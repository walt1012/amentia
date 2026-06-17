use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use rusqlite::{params, Connection};

use crate::time::current_timestamp;
use crate::types::StoredThreadRecord;

pub(crate) fn import_legacy_threads_if_needed(
  connection: &Connection,
  legacy_runtime_state_path: &Path,
) -> Result<()> {
  let existing_count: i64 =
    connection.query_row("SELECT COUNT(*) FROM threads", [], |row| row.get(0))?;
  if existing_count > 0 || !legacy_runtime_state_path.exists() {
    return Ok(());
  }

  let legacy_content = fs::read_to_string(legacy_runtime_state_path).with_context(|| {
    format!(
      "failed to read legacy runtime state from {}",
      legacy_runtime_state_path.display()
    )
  })?;
  let legacy_threads = serde_json::from_str::<Vec<StoredThreadRecord>>(&legacy_content)
    .with_context(|| {
      format!(
        "failed to parse legacy runtime state from {}",
        legacy_runtime_state_path.display()
      )
    })?;

  for thread in legacy_threads {
    let items_json =
      serde_json::to_string(&thread.items).context("failed to serialize migrated items")?;
    let workspace = thread
      .workspace
      .clone()
      .or(thread.summary.workspace.clone());
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
      ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
      params![
        thread.summary.id,
        thread.summary.title,
        thread.summary.status,
        thread.turn_count as i64,
        items_json,
        workspace
          .as_ref()
          .map(|workspace| workspace.root_path.clone()),
        workspace
          .as_ref()
          .map(|workspace| workspace.display_name.clone()),
        current_timestamp()?,
      ],
    )?;
  }

  Ok(())
}
