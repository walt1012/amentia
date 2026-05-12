use pith_plugin_host::PluginCommandEntry as HostPluginCommandEntry;

use super::plugin_command_types::{
  PluginConnectorCredentialProviderRef, PluginConnectorExecutionRef,
};
use super::plugin_connector_requirements::required_auth_connectors;
use crate::runtime_plugins::RuntimePluginState;

const LOCAL_CREDENTIAL_PROVIDER: &str = "pith.localCredentialProvider";

pub(super) fn build_command_connector_refs(
  command: &HostPluginCommandEntry,
  plugin_state: &RuntimePluginState,
) -> Vec<PluginConnectorExecutionRef> {
  required_auth_connectors(command, plugin_state)
    .into_iter()
    .filter_map(|connector| {
      let credential = plugin_state.connector_credential(&connector.connector_id)?;
      Some(PluginConnectorExecutionRef {
        connector_id: connector.connector_id.clone(),
        service: connector.service,
        credential_provider: PluginConnectorCredentialProviderRef {
          provider: LOCAL_CREDENTIAL_PROVIDER.to_string(),
          handle: connector.connector_id,
          store: credential.credential_store.clone(),
          label: credential.credential_label.clone(),
          authorized_at: credential.authorized_at,
        },
      })
    })
    .collect()
}
