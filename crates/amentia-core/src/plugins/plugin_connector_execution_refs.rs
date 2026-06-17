use amentia_plugin_host::{
  PluginCommandEntry as HostPluginCommandEntry, PluginConnectorEntry as HostPluginConnectorEntry,
};

use super::plugin_command_types::{
  PluginConnectorCredentialProviderRef, PluginConnectorExecutionRef, NO_CREDENTIAL_PROVIDER,
};
use super::plugin_connector_requirements::{
  command_connector_requirements, connector_requires_local_secret,
};
use crate::runtime_plugins::RuntimePluginState;

const LOCAL_CREDENTIAL_PROVIDER: &str = "amentia.localCredentialProvider";
pub(super) const CONNECTOR_AUTH_REPAIR_HINT: &str =
  "Authorize the connection before running this action.";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PluginConnectorExecutionRefError {
  pub(super) connector_id: String,
  pub(super) message: String,
  pub(super) repair_hint: &'static str,
}

pub(super) fn build_command_connector_refs(
  command: &HostPluginCommandEntry,
  plugin_state: &RuntimePluginState,
) -> Result<Vec<PluginConnectorExecutionRef>, PluginConnectorExecutionRefError> {
  let mut connector_refs = vec![];
  for (index, connector) in command_connector_requirements(command, plugin_state)
    .scoped_connectors
    .into_iter()
    .enumerate()
  {
    if !connector.auth_required {
      connector_refs.push(no_credential_connector_ref(connector));
      continue;
    }
    let Some(credential) = plugin_state.connector_credential(&connector.connector_id) else {
      return Err(missing_connector_credential(
        command,
        &connector.connector_id,
      ));
    };
    if connector_requires_local_secret(&connector) && credential.credential_secret.is_none() {
      return Err(missing_connector_credential_secret(
        command,
        &connector.connector_id,
      ));
    }
    connector_refs.push(PluginConnectorExecutionRef {
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
    });
  }
  Ok(connector_refs)
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

fn missing_connector_credential(
  command: &HostPluginCommandEntry,
  connector_id: &str,
) -> PluginConnectorExecutionRefError {
  PluginConnectorExecutionRefError {
    connector_id: connector_id.to_string(),
    message: format!(
      "Plugin command `{}` requires authorizing connector `{}` first.",
      command.command_id, connector_id
    ),
    repair_hint: CONNECTOR_AUTH_REPAIR_HINT,
  }
}

fn missing_connector_credential_secret(
  command: &HostPluginCommandEntry,
  connector_id: &str,
) -> PluginConnectorExecutionRefError {
  PluginConnectorExecutionRefError {
    connector_id: connector_id.to_string(),
    message: format!(
      "Plugin command `{}` requires authorizing connector `{}` with a local secret first.",
      command.command_id, connector_id
    ),
    repair_hint: CONNECTOR_AUTH_REPAIR_HINT,
  }
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
