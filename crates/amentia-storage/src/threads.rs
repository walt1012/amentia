use amentia_protocol::{ThreadSummary, TimelineItem, WorkspaceSummary};
use anyhow::{Context, Result};
use rusqlite::params;

use crate::time::current_timestamp;
use crate::types::StoredThreadRecord;
use crate::RuntimeStore;

impl RuntimeStore {
  pub fn load_threads(&self) -> Result<Vec<StoredThreadRecord>> {
    let connection = self.open_connection()?;

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

      let workspace = match (
        row.get::<_, Option<String>>(5)?,
        row.get::<_, Option<String>>(6)?,
      ) {
        (Some(root_path), Some(display_name)) => Some(WorkspaceSummary {
          root_path,
          display_name,
        }),
        _ => None,
      };

      Ok(StoredThreadRecord {
        summary: ThreadSummary {
          id: row.get(0)?,
          title: row.get(1)?,
          status: row.get(2)?,
          workspace: workspace.clone(),
        },
        turn_count: row.get::<_, i64>(3)? as usize,
        items,
        workspace,
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
      let workspace = thread
        .workspace
        .clone()
        .or(thread.summary.workspace.clone());

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

    transaction.commit()?;
    Ok(())
  }
}
