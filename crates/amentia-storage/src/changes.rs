use anyhow::Result;
use rusqlite::params;

use crate::time::current_timestamp;
use crate::types::StoredWorkspaceChangeRecord;
use crate::RuntimeStore;

impl RuntimeStore {
  pub fn save_workspace_change(&self, change: &StoredWorkspaceChangeRecord) -> Result<()> {
    let connection = self.open_connection()?;
    connection.execute(
      "INSERT INTO workspace_changes (
        id,
        thread_id,
        approval_id,
        workspace_root_path,
        relative_path,
        action,
        previous_content,
        next_content,
        created_at,
        reverted_at
      ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
      ON CONFLICT(id) DO UPDATE SET
        thread_id = excluded.thread_id,
        approval_id = excluded.approval_id,
        workspace_root_path = excluded.workspace_root_path,
        relative_path = excluded.relative_path,
        action = excluded.action,
        previous_content = excluded.previous_content,
        next_content = excluded.next_content,
        reverted_at = excluded.reverted_at",
      params![
        &change.id,
        &change.thread_id,
        &change.approval_id,
        &change.workspace_root_path,
        &change.relative_path,
        &change.action,
        &change.previous_content,
        &change.next_content,
        current_timestamp()?,
        &change.reverted_at,
      ],
    )?;

    Ok(())
  }

  pub fn load_workspace_changes_for_thread(
    &self,
    thread_id: &str,
  ) -> Result<Vec<StoredWorkspaceChangeRecord>> {
    let connection = self.open_connection()?;
    let mut statement = connection.prepare(
      "SELECT id, thread_id, approval_id, workspace_root_path, relative_path, action,
        previous_content, next_content, reverted_at
       FROM workspace_changes
       WHERE thread_id = ?1
       ORDER BY created_at ASC, id ASC",
    )?;

    let rows = statement.query_map(params![thread_id], |row| {
      Ok(StoredWorkspaceChangeRecord {
        id: row.get(0)?,
        thread_id: row.get(1)?,
        approval_id: row.get(2)?,
        workspace_root_path: row.get(3)?,
        relative_path: row.get(4)?,
        action: row.get(5)?,
        previous_content: row.get(6)?,
        next_content: row.get(7)?,
        reverted_at: row.get(8)?,
      })
    })?;

    Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
  }

  pub fn mark_workspace_change_reverted(&self, change_id: &str) -> Result<()> {
    let connection = self.open_connection()?;
    connection.execute(
      "UPDATE workspace_changes
       SET reverted_at = ?2
       WHERE id = ?1",
      params![change_id, current_timestamp()?],
    )?;

    Ok(())
  }

  pub fn delete_workspace_changes_for_thread(&self, thread_id: &str) -> Result<usize> {
    let connection = self.open_connection()?;
    let deleted = connection.execute(
      "DELETE FROM workspace_changes WHERE thread_id = ?1",
      params![thread_id],
    )?;

    Ok(deleted)
  }
}
