use amentia_protocol::{ThreadSummary, TimelineItem, WorkspaceSummary};
use anyhow::{Context, Result};
use rusqlite::params;

use crate::time::current_timestamp;
use crate::types::{StoredApprovalRecord, StoredThreadRecord};
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

    replace_threads(&transaction, threads)?;
    transaction.commit()?;
    Ok(())
  }

  pub fn save_runtime_after_thread_delete(
    &self,
    threads: &[StoredThreadRecord],
    pending_approvals: &[StoredApprovalRecord],
    deleted_thread_id: &str,
  ) -> Result<()> {
    let mut connection = self.open_connection()?;
    let transaction = connection.transaction()?;

    replace_threads(&transaction, threads)?;
    transaction.execute(
      "DELETE FROM approvals WHERE decision IS NULL OR thread_id = ?1",
      params![deleted_thread_id],
    )?;
    for approval in pending_approvals {
      if approval.thread_id == deleted_thread_id {
        continue;
      }
      insert_pending_approval(&transaction, approval)?;
    }
    transaction.execute(
      "DELETE FROM workspace_changes WHERE thread_id = ?1",
      params![deleted_thread_id],
    )?;

    transaction.commit()?;
    Ok(())
  }
}

fn replace_threads(
  transaction: &rusqlite::Transaction<'_>,
  threads: &[StoredThreadRecord],
) -> Result<()> {
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

  Ok(())
}

fn insert_pending_approval(
  transaction: &rusqlite::Transaction<'_>,
  approval: &StoredApprovalRecord,
) -> Result<()> {
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

  Ok(())
}
