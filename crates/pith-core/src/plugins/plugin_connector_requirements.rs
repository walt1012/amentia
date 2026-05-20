use pith_plugin_host::{
  PluginCommandEntry as HostPluginCommandEntry, PluginConnectorEntry as HostPluginConnectorEntry,
};

use crate::runtime_plugins::RuntimePluginState;

#[derive(Debug, Clone)]
pub(super) struct PluginCommandConnectorRequirements {
  pub(super) scoped_connectors: Vec<HostPluginConnectorEntry>,
  pub(super) connectors: Vec<HostPluginConnectorEntry>,
  pub(super) missing_connector_ids: Vec<String>,
  pub(super) connector_backed: bool,
}

pub(super) fn command_connector_requirements(
  command: &HostPluginCommandEntry,
  plugin_state: &RuntimePluginState,
) -> PluginCommandConnectorRequirements {
  let plugin_connectors = plugin_state
    .connector_entries()
    .into_iter()
    .filter(|connector| connector.plugin_id == command.plugin_id)
    .collect::<Vec<_>>();
  let auth_connectors = plugin_connectors
    .iter()
    .filter(|connector| connector.auth_required)
    .cloned()
    .collect::<Vec<_>>();
  let Some(declared_connector_ids) = command
    .execution
    .as_ref()
    .and_then(|execution| execution.connector_ids.as_ref())
  else {
    return PluginCommandConnectorRequirements {
      connector_backed: !plugin_connectors.is_empty(),
      scoped_connectors: plugin_connectors,
      connectors: auth_connectors,
      missing_connector_ids: vec![],
    };
  };

  let mut scoped_connectors = vec![];
  let mut required_connectors = vec![];
  let mut missing_connector_ids = vec![];
  let mut connector_backed = false;
  for connector_id in declared_connector_ids {
    let resolved_connector_id = qualified_connector_id(&command.plugin_id, connector_id);
    if let Some(connector) = plugin_connectors.iter().find(|connector| {
      connector.connector_id == connector_id.as_str()
        || connector.connector_id == resolved_connector_id.as_str()
    }) {
      connector_backed = true;
      if !scoped_connectors
        .iter()
        .any(|existing: &HostPluginConnectorEntry| existing.connector_id == connector.connector_id)
      {
        scoped_connectors.push(connector.clone());
      }
      if !connector.auth_required {
        continue;
      }
      if !required_connectors
        .iter()
        .any(|existing: &HostPluginConnectorEntry| existing.connector_id == connector.connector_id)
      {
        required_connectors.push(connector.clone());
      }
    } else {
      if !missing_connector_ids
        .iter()
        .any(|existing| existing == &resolved_connector_id)
      {
        missing_connector_ids.push(resolved_connector_id);
      }
    }
  }

  PluginCommandConnectorRequirements {
    scoped_connectors,
    connectors: required_connectors,
    missing_connector_ids,
    connector_backed,
  }
}

fn qualified_connector_id(plugin_id: &str, connector_id: &str) -> String {
  if connector_id.contains("::") {
    connector_id.to_string()
  } else {
    format!("{plugin_id}::{connector_id}")
  }
}
