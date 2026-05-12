use pith_plugin_host::{
  PluginCommandEntry as HostPluginCommandEntry, PluginConnectorEntry as HostPluginConnectorEntry,
};

use crate::runtime_plugins::RuntimePluginState;

pub(super) fn required_auth_connectors(
  command: &HostPluginCommandEntry,
  plugin_state: &RuntimePluginState,
) -> Vec<HostPluginConnectorEntry> {
  plugin_state
    .connector_entries()
    .into_iter()
    .filter(|connector| connector.plugin_id == command.plugin_id && connector.auth_required)
    .collect()
}
