use anyhow::Result;
use rusqlite::{params, Connection};

use crate::time::current_timestamp;

const SCHEMA_VERSION: i64 = 7;

#[derive(Debug, Clone, Copy)]
struct SchemaMigration {
  version: i64,
  name: &'static str,
  sql: &'static str,
}

const SCHEMA_MIGRATIONS: [SchemaMigration; 7] = [
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
  SchemaMigration {
    version: 7,
    name: "plugin_connector_credentials",
    sql: "
      CREATE TABLE IF NOT EXISTS plugin_connector_credentials (
        connector_id TEXT PRIMARY KEY,
        plugin_id TEXT NOT NULL,
        credential_store TEXT NOT NULL,
        credential_label TEXT NOT NULL,
        authorized_at INTEGER NOT NULL,
        updated_at INTEGER NOT NULL
      );
      CREATE INDEX IF NOT EXISTS idx_plugin_connector_credentials_plugin_id
      ON plugin_connector_credentials(plugin_id ASC, connector_id ASC);
    ",
  },
];

pub(crate) fn ensure_schema(connection: &Connection) -> Result<()> {
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
