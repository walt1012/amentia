use std::env;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use cavell_protocol::{ThreadSummary, TimelineItem, WorkspaceSummary};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

const SCHEMA_VERSION: i64 = 2;

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
      database_path: "~/Library/Application Support/Cavell/storage/cavell.db".to_string(),
      artifacts_path: "~/Library/Application Support/Cavell/artifacts".to_string(),
      plugins_path: "~/Library/Application Support/Cavell/plugins".to_string(),
      runtime_state_path: "~/Library/Application Support/Cavell/storage/threads.json".to_string(),
    }
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredThreadRecord {
  pub summary: ThreadSummary,
  pub turn_count: usize,
  pub items: Vec<TimelineItem>,
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
      "SELECT id, title, status, turn_count, items_json
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
          updated_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
          &thread.summary.id,
          &thread.summary.title,
          &thread.summary.status,
          thread.turn_count as i64,
          items_json,
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
      return Ok(());
    }

    connection.execute_batch(
      "
      BEGIN;
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
      PRAGMA user_version = 2;
      COMMIT;
      ",
    )?;

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
          updated_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
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

fn default_database_path() -> Result<PathBuf> {
  if let Ok(custom_dir) = env::var("CAVELL_DATA_DIR") {
    return Ok(PathBuf::from(custom_dir).join("cavell.db"));
  }

  if let Ok(home_dir) = env::var("HOME") {
    return Ok(PathBuf::from(home_dir).join(".cavell").join("cavell.db"));
  }

  if let Ok(home_dir) = env::var("USERPROFILE") {
    return Ok(PathBuf::from(home_dir).join(".cavell").join("cavell.db"));
  }

  Ok(
    env::current_dir()
      .context("failed to read current directory for database path")?
      .join(".cavell")
      .join("cavell.db"),
  )
}

fn default_runtime_state_path() -> Result<PathBuf> {
  if let Ok(custom_dir) = env::var("CAVELL_DATA_DIR") {
    return Ok(PathBuf::from(custom_dir).join("threads.json"));
  }

  if let Ok(home_dir) = env::var("HOME") {
    return Ok(PathBuf::from(home_dir).join(".cavell").join("threads.json"));
  }

  if let Ok(home_dir) = env::var("USERPROFILE") {
    return Ok(PathBuf::from(home_dir).join(".cavell").join("threads.json"));
  }

  Ok(
    env::current_dir()
      .context("failed to read current directory for runtime state")?
      .join(".cavell")
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
    let path = env::temp_dir().join(format!("cavell-storage-{label}-{unique}"));
    fs::create_dir_all(&path).expect("create temp directory");
    path
  }

  #[test]
  fn sqlite_store_round_trips_threads_and_workspace() {
    let root = create_temp_directory("sqlite-roundtrip");
    let store = FileThreadStore::new(root.join("cavell.db"), root.join("threads.json"));

    store
      .save_workspace(&WorkspaceSummary {
        root_path: "/tmp/cavell".to_string(),
        display_name: "cavell".to_string(),
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
      }])
      .expect("save threads");

    let workspace = store.load_workspace().expect("load workspace");
    let threads = store.load_threads().expect("load threads");

    fs::remove_dir_all(&root).expect("cleanup temp directory");

    assert_eq!(workspace.expect("workspace").display_name, "cavell");
    assert_eq!(threads.len(), 1);
    assert_eq!(threads[0].turn_count, 3);
    assert_eq!(threads[0].summary.id, "thread-1");
  }

  #[test]
  fn sqlite_store_round_trips_pending_approvals_and_resolution_audit() {
    let root = create_temp_directory("approval-roundtrip");
    let store = FileThreadStore::new(root.join("cavell.db"), root.join("threads.json"));
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
    let database_path = root.join("cavell.db");
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
}
