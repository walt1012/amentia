use amentia_plugin_host::{
  PluginCommandEntry as HostPluginCommandEntry, PluginConnectorEntry as HostPluginConnectorEntry,
};

use super::plugin_command_types::{
  PluginConnectorCredentialProviderRef, PluginConnectorExecutionRef, NO_CREDENTIAL_PROVIDER,
};
use super::plugin_connector_requirements::command_connector_requirements;
use crate::runtime_plugins::RuntimePluginState;

const LOCAL_CREDENTIAL_PROVIDER: &str = "amentia.localCredentialProvider";

pub(super) fn build_command_connector_refs(
  command: &HostPluginCommandEntry,
  plugin_state: &RuntimePluginState,
) -> Vec<PluginConnectorExecutionRef> {
  command_connector_requirements(command, plugin_state)
    .scoped_connectors
    .into_iter()
    .enumerate()
    .filter_map(|(index, connector)| {
      if !connector.auth_required {
        return Some(no_credential_connector_ref(connector));
      }
      let credential = plugin_state.connector_credential(&connector.connector_id)?;
      Some(PluginConnectorExecutionRef {
        connector_id: connector.connector_id.clone(),
        service: connector.service,
        credential_provider: PluginConnectorCredentialProviderRef {
          provider: LOCAL_CREDENTIAL_PROVIDER.to_string(),
          handle: connector.connector_id,
          store: credential.credential_store.clone(),
          label: credential.credential_label.clone(),
          env_key: credential
            .credential_secret
            .as_ref()
            .map(|_| credential_env_key(&credential.connector_id, index)),
          authorized_at: credential.authorized_at,
        },
        credential_secret: credential.credential_secret.clone(),
      })
    })
    .collect()
}

fn no_credential_connector_ref(connector: HostPluginConnectorEntry) -> PluginConnectorExecutionRef {
  PluginConnectorExecutionRef {
    connector_id: connector.connector_id.clone(),
    service: connector.service,
    credential_provider: PluginConnectorCredentialProviderRef {
      provider: NO_CREDENTIAL_PROVIDER.to_string(),
      handle: connector.connector_id,
      store: connector
        .credential_store
        .unwrap_or_else(|| "none".to_string()),
      label: "No credential required".to_string(),
      env_key: None,
      authorized_at: 0,
    },
    credential_secret: None,
  }
}

fn credential_env_key(connector_id: &str, index: usize) -> String {
  let suffix = connector_id
    .chars()
    .map(|character| {
      if character.is_ascii_alphanumeric() {
        character.to_ascii_uppercase()
      } else {
        '_'
      }
    })
    .collect::<String>();
  format!("AMENTIA_PLUGIN_CREDENTIAL_{}_{suffix}", index + 1)
}

#[cfg(test)]
mod tests {
  use super::credential_env_key;

  #[test]
  fn credential_env_keys_include_stable_run_index_to_avoid_collisions() {
    assert_eq!(
      credential_env_key("notion-mcp::notion", 0),
      "AMENTIA_PLUGIN_CREDENTIAL_1_NOTION_MCP__NOTION"
    );
    assert_ne!(
      credential_env_key("alpha-beta", 0),
      credential_env_key("alpha_beta", 1)
    );
  }
}
