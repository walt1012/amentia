use std::collections::HashMap;

use anyhow::Result;

use crate::plugin_catalog_state::{apply_plugin_states, load_plugin_catalog};
use crate::runtime_context::RuntimeContext;
use crate::runtime_plugins::PluginConnectorCredentialState;

impl RuntimeContext {
  pub(crate) fn persist_plugin_enabled(&self, plugin_id: &str, enabled: bool) -> Result<()> {
    self
      .persistence_state
      .save_plugin_enabled(plugin_id, enabled)
  }

  pub(crate) fn delete_plugin_state(&self, plugin_id: &str) -> Result<()> {
    self.persistence_state.delete_plugin_state(plugin_id)
  }

  pub(crate) fn persist_plugin_connector_credential(
    &self,
    credential: &PluginConnectorCredentialState,
  ) -> Result<()> {
    self
      .persistence_state
      .save_plugin_connector_credential(credential)
  }

  pub(crate) fn delete_plugin_connector_credential(&self, connector_id: &str) -> Result<()> {
    self
      .persistence_state
      .delete_plugin_connector_credential(connector_id)
  }

  pub(crate) fn delete_plugin_connector_credentials_for_plugin(
    &self,
    plugin_id: &str,
  ) -> Result<()> {
    self
      .persistence_state
      .delete_plugin_connector_credentials_for_plugin(plugin_id)
  }

  fn persisted_plugin_states(&self) -> Result<HashMap<String, bool>> {
    self.persistence_state.load_plugin_states()
  }

  pub(crate) fn refresh_plugins(&mut self) -> Result<()> {
    let plugin_states = self.persisted_plugin_states()?;
    self.plugin_state.replace_catalog(apply_plugin_states(
      load_plugin_catalog(self.plugin_state.roots())?,
      &plugin_states,
    ));
    Ok(())
  }
}
