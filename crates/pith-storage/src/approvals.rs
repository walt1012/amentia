use anyhow::{Context, Result};
use rusqlite::params;

use crate::time::current_timestamp;
use crate::types::StoredApprovalRecord;
use crate::RuntimeStore;

impl RuntimeStore {
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

  pub fn delete_approvals_for_thread(&self, thread_id: &str) -> Result<usize> {
    let connection = self.open_connection()?;
    let deleted = connection.execute(
      "DELETE FROM approvals WHERE thread_id = ?1",
      params![thread_id],
    )?;

    Ok(deleted)
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
}
