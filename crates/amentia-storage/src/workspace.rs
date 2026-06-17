use amentia_protocol::WorkspaceSummary;
use anyhow::Result;
use rusqlite::{params, OptionalExtension};

use crate::time::current_timestamp;
use crate::RuntimeStore;

impl RuntimeStore {
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
}
