use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use pith_memory::MemoryNote;
use pith_protocol::{ThreadSummary, TimelineItem, WorkspaceSummary};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

const SCHEMA_VERSION: i64 = 6;

#[derive(Debug, Clone, Copy)]
struct SchemaMigration {
  version: i64,
  name: &'static str,
  sql: &'static str,
}

const SCHEMA_MIGRATIONS: [SchemaMigration; 6] = [
  SchemaMigration {
    version: 1,
    name: "initial_workspace_and_threads",
    sql: "
      CREATE TABLE IF NOT EXISTS workspace_state (
        id INTEGER PRIMARY KEY CHECK (id = 1),
        root_path TEXT NOT NULL,
        display_name TEXT NOT NULL,
        updated_at INTEGER NOT NULL
      );
      CREATE TABLE IF NOT EXISTS threads (
        id TEXT PRIMARY KEY,
        title TEXT NOT NULL,
        status TEXT NOT NULL,
        turn_count INTEGER NOT NULL,
        items_json TEXT NOT NULL,
        updated_at INTEGER NOT NULL
      );
    ",
  },
  SchemaMigration {
    version: 2,
    name: "approval_audit",
    sql: "
      CREATE TABLE IF NOT EXISTS approvals (
        id TEXT PRIMARY KEY,
        thread_id TEXT NOT NULL,
        action TEXT NOT NULL,
        title TEXT NOT NULL,
        relative_path TEXT NOT NULL,
        content TEXT,
        command TEXT,
        requested_at INTEGER NOT NULL,
        decision TEXT,
        resolved_at INTEGER
      );
    ",
  },
  SchemaMigration {
    version: 3,
    name: "runtime_indexes",
    sql: "
      CREATE INDEX IF NOT EXISTS idx_threads_updated_at
      ON threads(updated_at DESC, id ASC);
      CREATE INDEX IF NOT EXISTS idx_approvals_requested_at
      ON approvals(decision, requested_at ASC, id ASC);
    ",
  },
  SchemaMigration {
    version: 4,
    name: "memory_notes",
    sql: "
      CREATE TABLE IF NOT EXISTS memory_notes (
        id TEXT PRIMARY KEY,
        title TEXT NOT NULL,
        body TEXT NOT NULL,
        scope TEXT NOT NULL,
        source TEXT NOT NULL,
        created_at INTEGER NOT NULL,
        tags_json TEXT NOT NULL
      );
      CREATE INDEX IF NOT EXISTS idx_memory_notes_created_at
      ON memory_notes(created_at DESC, id ASC);
    ",
  },
  SchemaMigration {
    version: 5,
    name: "thread_workspace_binding",
    sql: "
      ALTER TABLE threads ADD COLUMN workspace_root_path TEXT;
      ALTER TABLE threads ADD COLUMN workspace_display_name TEXT;
    ",
  },
  SchemaMigration {
    version: 6,
    name: "plugin_state",
    sql: "
      CREATE TABLE IF NOT EXISTS plugin_state (
        plugin_id TEXT PRIMARY KEY,
        enabled INTEGER NOT NULL,
        updated_at INTEGER NOT NULL
      );
    ",
  },
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoragePaths {
  pub database_path: String,
  pub artifacts_path: String,
  pub plugins_path: String,
  pub runtime_state_path: String,
}

impl StoragePaths {
  pub fn application_support_defaults() -> Self {
    Self {
      database_path: "~/Library/Application Support/Pith/storage/pith.db".to_string(),
      artifacts_path: "~/Library/Application Support/Pith/artifacts".to_string(),
      plugins_path: "~/Library/Application Support/Pith/plugins".to_string(),
      runtime_state_path: "~/Library/Application Support/Pith/storage/threads.json".to_string(),
    }
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredThreadRecord {
  pub summary: ThreadSummary,
  pub turn_count: usize,
  pub items: Vec<TimelineItem>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub workspace: Option<WorkspaceSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredApprovalRecord {
  pub id: String,
  pub thread_id: String,
  pub action: String,
  pub title: String,
  pub relative_path: String,
  pub content: Option<String>,
  pub command: Option<String>,
}

#[derive(Debug, Clone)]
pub struct FileThreadStore {
  database_path: PathBuf,
  legacy_runtime_state_path: PathBuf,
}

impl FileThreadStore {
  pub fn new_default() -> Result<Self> {
    Ok(Self {
      database_path: default_database_path()?,
      legacy_runtime_state_path: default_runtime_state_path()?,
    })
  }

  pub fn new(database_path: PathBuf, legacy_runtime_state_path: PathBuf) -> Self {
    Self {
      database_path,
      legacy_runtime_state_path,
    }
  }

  pub fn load_threads(&self) -> Result<Vec<StoredThreadRecord>> {
    let connection = self.open_connection()?;
    self.import_legacy_threads_if_needed(&connection)?;

    let mut statement = connection.prepare(
      "SELECT id, title, status, turn_count, items_json, workspace_root_path, workspace_display_name
       FROM threads
       ORDER BY updated_at DESC, id ASC",
    )?;

    let rows = statement.query_map([], |row| {
      let items_json: String = row.get(4)?;
      let items = serde_json::from_str::<Vec<TimelineItem>>(&items_json).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(
          items_json.len(),
          rusqlite::types::Type::Text,
          Box::new(error),
        )
      })?;

      Ok(StoredThreadRecord {
        summary: ThreadSummary {
          id: row.get(0)?,
          title: row.get(1)?,
          status: row.get(2)?,
        },
        turn_count: row.get::<_, i64>(3)? as usize,
        items,
        workspace: match (
          row.get::<_, Option<String>>(5)?,
          row.get::<_, Option<String>>(6)?,
        ) {
          (Some(root_path), Some(display_name)) => Some(WorkspaceSummary {
            root_path,
            display_name,
          }),
          _ => None,
        },
      })
    })?;

    rows
      .collect::<std::result::Result<Vec<_>, _>>()
      .context("failed to load persisted thread records")
  }

  pub fn save_threads(&self, threads: &[StoredThreadRecord]) -> Result<()> {
    let mut connection = self.open_connection()?;
    let transaction = connection.transaction()?;

    transaction.execute("DELETE FROM threads", [])?;

    for thread in threads {
      let items_json =
        serde_json::to_string(&thread.items).context("failed to serialize timeline items")?;

      transaction.execute(
        "INSERT INTO threads (
          id,
          title,
          status,
          turn_count,
          items_json,
          workspace_root_path,
          workspace_display_name,
          updated_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
          &thread.summary.id,
          &thread.summary.title,
          &thread.summary.status,
          thread.turn_count as i64,
          items_json,
          thread
            .workspace
            .as_ref()
            .map(|workspace| workspace.root_path.clone()),
          thread
            .workspace
            .as_ref()
            .map(|workspace| workspace.display_name.clone()),
          current_timestamp()?,
        ],
      )?;
    }

    transaction.commit()?;
    Ok(())
  }

  pub fn load_workspace(&self) -> Result<Option<WorkspaceSummary>> {
    let connection = self.open_connection()?;
    let workspace = connection
      .query_row(
        "SELECT root_path, display_name
         FROM workspace_state
         WHERE id = 1",
        [],
        |row| {
          Ok(WorkspaceSummary {
            root_path: row.get(0)?,
            display_name: row.get(1)?,
          })
        },
      )
      .optional()?;

    Ok(workspace)
  }

  pub fn load_pending_approvals(&self) -> Result<Vec<StoredApprovalRecord>> {
    let connection = self.open_connection()?;
    let mut statement = connection.prepare(
      "SELECT id, thread_id, action, title, relative_path, content, command
       FROM approvals
       WHERE decision IS NULL
       ORDER BY requested_at ASC, id ASC",
    )?;

    let rows = statement.query_map([], |row| {
      Ok(StoredApprovalRecord {
        id: row.get(0)?,
        thread_id: row.get(1)?,
        action: row.get(2)?,
        title: row.get(3)?,
        relative_path: row.get(4)?,
        content: row.get(5)?,
        command: row.get(6)?,
      })
    })?;

    rows
      .collect::<std::result::Result<Vec<_>, _>>()
      .context("failed to load pending approval records")
  }

  pub fn load_memory_notes(&self, limit: usize) -> Result<Vec<MemoryNote>> {
    let connection = self.open_connection()?;
    let mut statement = connection.prepare(
      "SELECT id, title, body, scope, source, created_at, tags_json
       FROM memory_notes
       ORDER BY created_at DESC, id ASC
       LIMIT ?1",
    )?;

    let rows = statement.query_map([limit as i64], |row| {
      let tags_json: String = row.get(6)?;
      let tags = serde_json::from_str::<Vec<String>>(&tags_json).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(
          tags_json.len(),
          rusqlite::types::Type::Text,
          Box::new(error),
        )
      })?;

      Ok(MemoryNote {
        id: row.get(0)?,
        title: row.get(1)?,
        body: row.get(2)?,
        scope: row.get(3)?,
        source: row.get(4)?,
        created_at: row.get(5)?,
        tags,
      })
    })?;

    rows
      .collect::<std::result::Result<Vec<_>, _>>()
      .context("failed to load memory notes")
  }

  pub fn load_plugin_states(&self) -> Result<HashMap<String, bool>> {
    let connection = self.open_connection()?;
    let mut statement =
      connection.prepare("SELECT plugin_id, enabled FROM plugin_state ORDER BY plugin_id ASC")?;
    let rows = statement.query_map([], |row| {
      Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? != 0))
    })?;

    Ok(
      rows
        .collect::<std::result::Result<Vec<_>, _>>()?
        .into_iter()
        .collect(),
    )
  }

  pub fn save_pending_approvals(&self, approvals: &[StoredApprovalRecord]) -> Result<()> {
    let mut connection = self.open_connection()?;
    let transaction = connection.transaction()?;
    transaction.execute("DELETE FROM approvals WHERE decision IS NULL", [])?;

    for approval in approvals {
      transaction.execute(
        "INSERT INTO approvals (
          id,
          thread_id,
          action,
          title,
          relative_path,
          content,
          command,
          requested_at,
          decision,
          resolved_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, NULL, NULL)",
        params![
          &approval.id,
          &approval.thread_id,
          &approval.action,
          &approval.title,
          &approval.relative_path,
          &approval.content,
          &approval.command,
          current_timestamp()?,
        ],
      )?;
    }

    transaction.commit()?;
    Ok(())
  }

  pub fn resolve_approval(&self, approval: &StoredApprovalRecord, decision: &str) -> Result<()> {
    let connection = self.open_connection()?;
    connection.execute(
      "INSERT INTO approvals (
        id,
        thread_id,
        action,
        title,
        relative_path,
        content,
        command,
        requested_at,
        decision,
        resolved_at
      ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
      ON CONFLICT(id) DO UPDATE SET
        thread_id = excluded.thread_id,
        action = excluded.action,
        title = excluded.title,
        relative_path = excluded.relative_path,
        content = excluded.content,
        command = excluded.command,
        decision = excluded.decision,
        resolved_at = excluded.resolved_at",
      params![
        &approval.id,
        &approval.thread_id,
        &approval.action,
        &approval.title,
        &approval.relative_path,
        &approval.content,
        &approval.command,
        current_timestamp()?,
        decision,
        current_timestamp()?,
      ],
    )?;

    Ok(())
  }

  pub fn next_approval_sequence(&self) -> Result<usize> {
    let connection = self.open_connection()?;
    let mut statement = connection.prepare("SELECT id FROM approvals ORDER BY id ASC")?;
    let rows = statement.query_map([], |row| row.get::<_, String>(0))?;

    let max_sequence = rows
      .collect::<std::result::Result<Vec<_>, _>>()?
      .into_iter()
      .filter_map(|id| {
        id.strip_prefix("approval-")
          .and_then(|suffix| suffix.parse::<usize>().ok())
      })
      .max()
      .unwrap_or(0);

    Ok(max_sequence + 1)
  }

  pub fn save_memory_note(&self, note: &MemoryNote) -> Result<()> {
    let connection = self.open_connection()?;
    let tags_json =
      serde_json::to_string(&note.tags).context("failed to serialize memory note tags")?;
    connection.execute(
      "INSERT INTO memory_notes (
        id,
        title,
        body,
        scope,
        source,
        created_at,
        tags_json
      ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
      ON CONFLICT(id) DO UPDATE SET
        title = excluded.title,
        body = excluded.body,
        scope = excluded.scope,
        source = excluded.source,
        created_at = excluded.created_at,
        tags_json = excluded.tags_json",
      params![
        &note.id,
        &note.title,
        &note.body,
        &note.scope,
        &note.source,
        note.created_at,
        tags_json,
      ],
    )?;

    Ok(())
  }

  pub fn next_memory_sequence(&self) -> Result<usize> {
    let connection = self.open_connection()?;
    let mut statement = connection.prepare("SELECT id FROM memory_notes ORDER BY id ASC")?;
    let rows = statement.query_map([], |row| row.get::<_, String>(0))?;

    let max_sequence = rows
      .collect::<std::result::Result<Vec<_>, _>>()?
      .into_iter()
      .filter_map(|id| {
        id.strip_prefix("memory-")
          .and_then(|suffix| suffix.parse::<usize>().ok())
      })
      .max()
      .unwrap_or(0);

    Ok(max_sequence + 1)
  }

  pub fn save_workspace(&self, workspace: &WorkspaceSummary) -> Result<()> {
    let connection = self.open_connection()?;
    connection.execute(
      "INSERT INTO workspace_state (id, root_path, display_name, updated_at)
       VALUES (1, ?1, ?2, ?3)
       ON CONFLICT(id) DO UPDATE SET
         root_path = excluded.root_path,
         display_name = excluded.display_name,
         updated_at = excluded.updated_at",
      params![
        &workspace.root_path,
        &workspace.display_name,
        current_timestamp()?,
      ],
    )?;

    Ok(())
  }

  pub fn save_plugin_enabled(&self, plugin_id: &str, enabled: bool) -> Result<()> {
    let connection = self.open_connection()?;
    connection.execute(
      "INSERT INTO plugin_state (plugin_id, enabled, updated_at)
       VALUES (?1, ?2, ?3)
       ON CONFLICT(plugin_id) DO UPDATE SET
         enabled = excluded.enabled,
         updated_at = excluded.updated_at",
      params![plugin_id, if enabled { 1 } else { 0 }, current_timestamp()?],
    )?;

    Ok(())
  }

  fn open_connection(&self) -> Result<Connection> {
    if let Some(parent) = self.database_path.parent() {
      fs::create_dir_all(parent)
        .with_context(|| format!("failed to create storage directory {}", parent.display()))?;
    }

    let connection = Connection::open(&self.database_path)
      .with_context(|| format!("failed to open database {}", self.database_path.display()))?;
    self.ensure_schema(&connection)?;
    Ok(connection)
  }

  fn ensure_schema(&self, connection: &Connection) -> Result<()> {
    let version: i64 = connection.pragma_query_value(None, "user_version", |row| row.get(0))?;

    if version >= SCHEMA_VERSION {
      ensure_schema_migration_table(connection)?;
      return Ok(());
    }

    let transaction = connection.unchecked_transaction()?;
    ensure_schema_migration_table(&transaction)?;

    for migration in SCHEMA_MIGRATIONS {
      if migration.version <= version {
        backfill_schema_migration(&transaction, migration)?;
        continue;
      }

      if migration.version == 5 {
        add_column_if_missing(
          &transaction,
          "threads",
          "workspace_root_path",
          "ALTER TABLE threads ADD COLUMN workspace_root_path TEXT;",
        )?;
        add_column_if_missing(
          &transaction,
          "threads",
          "workspace_display_name",
          "ALTER TABLE threads ADD COLUMN workspace_display_name TEXT;",
        )?;
      } else {
        transaction.execute_batch(migration.sql)?;
      }
      record_schema_migration(&transaction, migration)?;
    }

    transaction.pragma_update(None, "user_version", SCHEMA_VERSION)?;
    transaction.commit()?;

    Ok(())
  }

  fn import_legacy_threads_if_needed(&self, connection: &Connection) -> Result<()> {
    let existing_count: i64 =
      connection.query_row("SELECT COUNT(*) FROM threads", [], |row| row.get(0))?;
    if existing_count > 0 || !self.legacy_runtime_state_path.exists() {
      return Ok(());
    }

    let legacy_content =
      fs::read_to_string(&self.legacy_runtime_state_path).with_context(|| {
        format!(
          "failed to read legacy runtime state from {}",
          self.legacy_runtime_state_path.display()
        )
      })?;
    let legacy_threads = serde_json::from_str::<Vec<StoredThreadRecord>>(&legacy_content)
      .with_context(|| {
        format!(
          "failed to parse legacy runtime state from {}",
          self.legacy_runtime_state_path.display()
        )
      })?;

    for thread in legacy_threads {
      let items_json =
        serde_json::to_string(&thread.items).context("failed to serialize migrated items")?;
      connection.execute(
        "INSERT INTO threads (
          id,
          title,
          status,
          turn_count,
          items_json,
          workspace_root_path,
          workspace_display_name,
          updated_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, NULL, NULL, ?6)",
        params![
          thread.summary.id,
          thread.summary.title,
          thread.summary.status,
          thread.turn_count as i64,
          items_json,
          current_timestamp()?,
        ],
      )?;
    }

    Ok(())
  }
}

fn ensure_schema_migration_table(connection: &Connection) -> Result<()> {
  connection.execute_batch(
    "
    CREATE TABLE IF NOT EXISTS schema_migrations (
      version INTEGER PRIMARY KEY,
      name TEXT NOT NULL,
      applied_at INTEGER NOT NULL
    );
    ",
  )?;

  Ok(())
}

fn add_column_if_missing(
  connection: &Connection,
  table: &str,
  column: &str,
  sql: &str,
) -> Result<()> {
  if table_has_column(connection, table, column)? {
    return Ok(());
  }

  connection.execute_batch(sql)?;
  Ok(())
}

fn table_has_column(connection: &Connection, table: &str, column: &str) -> Result<bool> {
  let pragma = format!("PRAGMA table_info({table})");
  let mut statement = connection.prepare(&pragma)?;
  let rows = statement.query_map([], |row| row.get::<_, String>(1))?;
  Ok(
    rows
      .collect::<std::result::Result<Vec<_>, _>>()?
      .into_iter()
      .any(|name| name == column),
  )
}

fn backfill_schema_migration(connection: &Connection, migration: SchemaMigration) -> Result<()> {
  connection.execute(
    "INSERT OR IGNORE INTO schema_migrations (version, name, applied_at)
     VALUES (?1, ?2, ?3)",
    params![migration.version, migration.name, current_timestamp()?],
  )?;

  Ok(())
}

fn record_schema_migration(connection: &Connection, migration: SchemaMigration) -> Result<()> {
  connection.execute(
    "INSERT INTO schema_migrations (version, name, applied_at)
     VALUES (?1, ?2, ?3)",
    params![migration.version, migration.name, current_timestamp()?],
  )?;

  Ok(())
}

fn default_database_path() -> Result<PathBuf> {
  if let Ok(custom_dir) = env::var("PITH_DATA_DIR") {
    return Ok(PathBuf::from(custom_dir).join("pith.db"));
  }

  if let Ok(home_dir) = env::var("HOME") {
    return Ok(PathBuf::from(home_dir).join(".pith").join("pith.db"));
  }

  if let Ok(home_dir) = env::var("USERPROFILE") {
    return Ok(PathBuf::from(home_dir).join(".pith").join("pith.db"));
  }

  Ok(
    env::current_dir()
      .context("failed to read current directory for database path")?
      .join(".pith")
      .join("pith.db"),
  )
}

fn default_runtime_state_path() -> Result<PathBuf> {
  if let Ok(custom_dir) = env::var("PITH_DATA_DIR") {
    return Ok(PathBuf::from(custom_dir).join("threads.json"));
  }

  if let Ok(home_dir) = env::var("HOME") {
    return Ok(PathBuf::from(home_dir).join(".pith").join("threads.json"));
  }

  if let Ok(home_dir) = env::var("USERPROFILE") {
    return Ok(PathBuf::from(home_dir).join(".pith").join("threads.json"));
  }

  Ok(
    env::current_dir()
      .context("failed to read current directory for runtime state")?
      .join(".pith")
      .join("threads.json"),
  )
}

fn current_timestamp() -> Result<i64> {
  Ok(
    SystemTime::now()
      .duration_since(UNIX_EPOCH)
      .context("system time is earlier than the unix epoch")?
      .as_secs() as i64,
  )
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::time::{SystemTime, UNIX_EPOCH};

  fn create_temp_directory(label: &str) -> PathBuf {
    let unique = SystemTime::now()
      .duration_since(UNIX_EPOCH)
      .expect("system time")
      .as_nanos();
    let path = env::temp_dir().join(format!("pith-storage-{label}-{unique}"));
    fs::create_dir_all(&path).expect("create temp directory");
    path
  }

  #[test]
  fn sqlite_store_round_trips_threads_and_workspace() {
    let root = create_temp_directory("sqlite-roundtrip");
    let store = FileThreadStore::new(root.join("pith.db"), root.join("threads.json"));

    store
      .save_workspace(&WorkspaceSummary {
        root_path: "/tmp/pith".to_string(),
        display_name: "pith".to_string(),
      })
      .expect("save workspace");
    store
      .save_threads(&[StoredThreadRecord {
        summary: ThreadSummary {
          id: "thread-1".to_string(),
          title: "Thread".to_string(),
          status: "ready".to_string(),
        },
        turn_count: 3,
        items: vec![TimelineItem {
          kind: "system".to_string(),
          title: "Thread Ready".to_string(),
          content: "Ready".to_string(),
          attributes: None,
        }],
        workspace: Some(WorkspaceSummary {
          root_path: "/tmp/pith".to_string(),
          display_name: "pith".to_string(),
        }),
      }])
      .expect("save threads");

    let workspace = store.load_workspace().expect("load workspace");
    let threads = store.load_threads().expect("load threads");

    fs::remove_dir_all(&root).expect("cleanup temp directory");

    assert_eq!(workspace.expect("workspace").display_name, "pith");
    assert_eq!(threads.len(), 1);
    assert_eq!(threads[0].turn_count, 3);
    assert_eq!(threads[0].summary.id, "thread-1");
    assert_eq!(
      threads[0]
        .workspace
        .as_ref()
        .expect("thread workspace")
        .display_name,
      "pith"
    );
  }

  #[test]
  fn sqlite_store_round_trips_pending_approvals_and_resolution_audit() {
    let root = create_temp_directory("approval-roundtrip");
    let store = FileThreadStore::new(root.join("pith.db"), root.join("threads.json"));
    let approval = StoredApprovalRecord {
      id: "approval-4".to_string(),
      thread_id: "thread-2".to_string(),
      action: "write_file".to_string(),
      title: "Write docs/output.txt".to_string(),
      relative_path: "docs/output.txt".to_string(),
      content: Some("hello".to_string()),
      command: None,
    };

    store
      .save_pending_approvals(std::slice::from_ref(&approval))
      .expect("save pending approvals");
    let pending = store
      .load_pending_approvals()
      .expect("load pending approvals");
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].id, "approval-4");

    store
      .resolve_approval(&approval, "approved")
      .expect("resolve approval");
    let reloaded_pending = store
      .load_pending_approvals()
      .expect("reload pending approvals");
    let next_sequence = store
      .next_approval_sequence()
      .expect("next approval sequence");

    fs::remove_dir_all(&root).expect("cleanup temp directory");

    assert!(reloaded_pending.is_empty());
    assert_eq!(next_sequence, 5);
  }

  #[test]
  fn sqlite_store_imports_legacy_json_threads() {
    let root = create_temp_directory("legacy-import");
    let database_path = root.join("pith.db");
    let legacy_path = root.join("threads.json");
    fs::write(
      &legacy_path,
      serde_json::to_string(&vec![StoredThreadRecord {
        summary: ThreadSummary {
          id: "thread-legacy".to_string(),
          title: "Legacy".to_string(),
          status: "ready".to_string(),
        },
        turn_count: 1,
        items: vec![],
        workspace: None,
      }])
      .expect("serialize legacy threads"),
    )
    .expect("write legacy threads");

    let store = FileThreadStore::new(database_path, legacy_path);
    let threads = store.load_threads().expect("load migrated threads");

    fs::remove_dir_all(&root).expect("cleanup temp directory");

    assert_eq!(threads.len(), 1);
    assert_eq!(threads[0].summary.id, "thread-legacy");
  }

  #[test]
  fn sqlite_store_migrates_existing_version_one_database() {
    let root = create_temp_directory("migrate-v1");
    let database_path = root.join("pith.db");
    let connection = Connection::open(&database_path).expect("open seed database");
    connection
      .execute_batch(
        "
        CREATE TABLE workspace_state (
          id INTEGER PRIMARY KEY CHECK (id = 1),
          root_path TEXT NOT NULL,
          display_name TEXT NOT NULL,
          updated_at INTEGER NOT NULL
        );
        CREATE TABLE threads (
          id TEXT PRIMARY KEY,
          title TEXT NOT NULL,
          status TEXT NOT NULL,
          turn_count INTEGER NOT NULL,
          items_json TEXT NOT NULL,
          updated_at INTEGER NOT NULL
        );
        INSERT INTO threads (id, title, status, turn_count, items_json, updated_at)
        VALUES ('thread-old', 'Old Thread', 'ready', 1, '[]', 1);
        PRAGMA user_version = 1;
        ",
      )
      .expect("seed version one schema");
    drop(connection);

    let store = FileThreadStore::new(database_path.clone(), root.join("threads.json"));
    let threads = store.load_threads().expect("load migrated threads");
    let pending_approvals = store
      .load_pending_approvals()
      .expect("load migrated approvals");
    let connection = Connection::open(&database_path).expect("reopen migrated database");
    let migration_versions: Vec<i64> = connection
      .prepare("SELECT version FROM schema_migrations ORDER BY version ASC")
      .expect("prepare schema migrations query")
      .query_map([], |row| row.get(0))
      .expect("query schema migrations")
      .collect::<std::result::Result<Vec<_>, _>>()
      .expect("collect schema migrations");
    let approval_indexes: Vec<String> = connection
      .prepare(
        "SELECT name
         FROM sqlite_master
         WHERE type = 'index' AND tbl_name = 'approvals'
         ORDER BY name ASC",
      )
      .expect("prepare approvals index query")
      .query_map([], |row| row.get(0))
      .expect("query approvals indexes")
      .collect::<std::result::Result<Vec<_>, _>>()
      .expect("collect approvals indexes");

    fs::remove_dir_all(&root).expect("cleanup temp directory");

    assert_eq!(threads.len(), 1);
    assert_eq!(threads[0].summary.id, "thread-old");
    assert!(pending_approvals.is_empty());
    assert_eq!(migration_versions, vec![1, 2, 3, 4, 5, 6]);
    assert!(approval_indexes.contains(&"idx_approvals_requested_at".to_string()));
  }

  #[test]
  fn sqlite_store_round_trips_memory_notes() {
    let root = create_temp_directory("memory-roundtrip");
    let store = FileThreadStore::new(root.join("pith.db"), root.join("threads.json"));
    let note = MemoryNote {
      id: "memory-7".to_string(),
      title: "Opened workspace pith".to_string(),
      body: "Pith opened the workspace at /tmp/pith.".to_string(),
      scope: "pith".to_string(),
      source: "workspace".to_string(),
      created_at: 7,
      tags: vec!["workspace".to_string(), "session".to_string()],
    };

    store.save_memory_note(&note).expect("save memory note");
    let notes = store.load_memory_notes(10).expect("load memory notes");
    let next_sequence = store.next_memory_sequence().expect("next memory sequence");

    fs::remove_dir_all(&root).expect("cleanup temp directory");

    assert_eq!(notes.len(), 1);
    assert_eq!(notes[0].id, "memory-7");
    assert_eq!(notes[0].tags, vec!["workspace", "session"]);
    assert_eq!(next_sequence, 8);
  }

  #[test]
  fn sqlite_store_round_trips_plugin_states() {
    let root = create_temp_directory("plugin-state");
    let store = FileThreadStore::new(root.join("pith.db"), root.join("threads.json"));

    store
      .save_plugin_enabled("workspace-notes", true)
      .expect("save plugin enabled");
    store
      .save_plugin_enabled("shell-recorder", false)
      .expect("save plugin disabled");
    let plugin_states = store.load_plugin_states().expect("load plugin states");

    fs::remove_dir_all(&root).expect("cleanup temp directory");

    assert_eq!(plugin_states.get("workspace-notes"), Some(&true));
    assert_eq!(plugin_states.get("shell-recorder"), Some(&false));
  }
}
