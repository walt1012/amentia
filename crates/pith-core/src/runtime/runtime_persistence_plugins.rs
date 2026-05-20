use std::collections::HashMap;

use anyhow::Result;
use pith_storage::{RuntimeStore, StoredPluginConnectorCredential};

use super::runtime_plugins::PluginConnectorCredentialState;

pub(super) fn save_plugin_enabled(
  store: Option<&RuntimeStore>,
  plugin_id: &str,
  enabled: bool,
) -> Result<()> {
  let Some(store) = store else {
    return Ok(());
  };

  store.save_plugin_enabled(plugin_id, enabled)
}

pub(super) fn delete_plugin_state(store: Option<&RuntimeStore>, plugin_id: &str) -> Result<()> {
  let Some(store) = store else {
    return Ok(());
  };

  store.delete_plugin_state(plugin_id)
}

pub(super) fn save_plugin_connector_credential(
  store: Option<&RuntimeStore>,
  credential: &PluginConnectorCredentialState,
) -> Result<()> {
  let Some(store) = store else {
    return Ok(());
  };

  store.save_plugin_connector_credential(&StoredPluginConnectorCredential::from(credential))
}

pub(super) fn delete_plugin_connector_credential(
  store: Option<&RuntimeStore>,
  connector_id: &str,
) -> Result<()> {
  let Some(store) = store else {
    return Ok(());
  };

  store.delete_plugin_connector_credential(connector_id)
}

pub(super) fn delete_plugin_connector_credentials_for_plugin(
  store: Option<&RuntimeStore>,
  plugin_id: &str,
) -> Result<()> {
  let Some(store) = store else {
    return Ok(());
  };

  store.delete_plugin_connector_credentials_for_plugin(plugin_id)
}

pub(super) fn load_plugin_states(store: Option<&RuntimeStore>) -> Result<HashMap<String, bool>> {
  let Some(store) = store else {
    return Ok(HashMap::new());
  };

  store.load_plugin_states()
}
