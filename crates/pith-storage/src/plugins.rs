use std::collections::HashMap;

use anyhow::Result;
use rusqlite::params;

use crate::time::current_timestamp;
use crate::RuntimeStore;

impl RuntimeStore {
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

  pub fn delete_plugin_state(&self, plugin_id: &str) -> Result<()> {
    let connection = self.open_connection()?;
    connection.execute(
      "DELETE FROM plugin_state WHERE plugin_id = ?1",
      params![plugin_id],
    )?;
    Ok(())
  }
}
