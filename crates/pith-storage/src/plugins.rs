use std::collections::HashMap;

use anyhow::Result;
use rusqlite::params;

use crate::time::current_timestamp;
use crate::{RuntimeStore, StoredPluginConnectorCredential};

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

  pub fn load_plugin_connector_credentials(&self) -> Result<Vec<StoredPluginConnectorCredential>> {
    let connection = self.open_connection()?;
    let mut statement = connection.prepare(
      "SELECT connector_id, plugin_id, credential_store, credential_label, authorized_at, updated_at
       FROM plugin_connector_credentials
       ORDER BY plugin_id ASC, connector_id ASC",
    )?;
    let rows = statement.query_map([], |row| {
      Ok(StoredPluginConnectorCredential {
        connector_id: row.get(0)?,
        plugin_id: row.get(1)?,
        credential_store: row.get(2)?,
        credential_label: row.get(3)?,
        authorized_at: row.get(4)?,
        updated_at: row.get(5)?,
      })
    })?;

    Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
  }

  pub fn save_plugin_connector_credential(
    &self,
    credential: &StoredPluginConnectorCredential,
  ) -> Result<()> {
    let connection = self.open_connection()?;
    connection.execute(
      "INSERT INTO plugin_connector_credentials (
         connector_id, plugin_id, credential_store, credential_label, authorized_at, updated_at
       )
       VALUES (?1, ?2, ?3, ?4, ?5, ?6)
       ON CONFLICT(connector_id) DO UPDATE SET
         plugin_id = excluded.plugin_id,
         credential_store = excluded.credential_store,
         credential_label = excluded.credential_label,
         authorized_at = excluded.authorized_at,
         updated_at = excluded.updated_at",
      params![
        &credential.connector_id,
        &credential.plugin_id,
        &credential.credential_store,
        &credential.credential_label,
        credential.authorized_at,
        credential.updated_at
      ],
    )?;

    Ok(())
  }

  pub fn delete_plugin_connector_credential(&self, connector_id: &str) -> Result<()> {
    let connection = self.open_connection()?;
    connection.execute(
      "DELETE FROM plugin_connector_credentials WHERE connector_id = ?1",
      params![connector_id],
    )?;
    Ok(())
  }

  pub fn delete_plugin_connector_credentials_for_plugin(&self, plugin_id: &str) -> Result<()> {
    let connection = self.open_connection()?;
    connection.execute(
      "DELETE FROM plugin_connector_credentials WHERE plugin_id = ?1",
      params![plugin_id],
    )?;
    Ok(())
  }
}
