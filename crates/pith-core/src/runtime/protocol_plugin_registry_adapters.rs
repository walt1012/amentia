use std::collections::HashMap;

use pith_plugin_host::{
  build_capability_registry, build_command_registry, build_hook_registry, PluginCatalogEntry,
};
use pith_protocol::{
  PluginCapabilityRegistryResult, PluginCapabilityRegistrySummary, PluginCommandRegistryResult,
  PluginConnectorRegistryResult, PluginHookRegistryResult,
};

use super::protocol_plugin_registry_mappers::{
  to_protocol_capability, to_protocol_plugin_command, to_protocol_plugin_connector,
  to_protocol_plugin_hook,
};
use super::runtime_plugins::RuntimePluginState;

pub(crate) fn build_protocol_capability_registry(
  plugins: &[PluginCatalogEntry],
) -> PluginCapabilityRegistryResult {
  let capabilities = build_capability_registry(plugins)
    .into_iter()
    .map(to_protocol_capability)
    .collect::<Vec<_>>();
  let enabled_plugin_count = plugins
    .iter()
    .filter(|plugin| plugin.status == "ready" && plugin.enabled)
    .count();
  let mut capability_counts_by_kind = HashMap::new();
  for capability in &capabilities {
    *capability_counts_by_kind
      .entry(capability.kind.clone())
      .or_insert(0) += 1;
  }

  PluginCapabilityRegistryResult {
    summary: PluginCapabilityRegistrySummary {
      enabled_plugin_count,
      total_capability_count: capabilities.len(),
      capability_counts_by_kind,
    },
    capabilities,
  }
}

pub(crate) fn build_protocol_command_registry(
  plugins: &[PluginCatalogEntry],
) -> PluginCommandRegistryResult {
  PluginCommandRegistryResult {
    commands: build_command_registry(plugins)
      .into_iter()
      .map(to_protocol_plugin_command)
      .collect(),
  }
}

pub(crate) fn build_protocol_connector_registry(
  plugin_state: &RuntimePluginState,
) -> PluginConnectorRegistryResult {
  PluginConnectorRegistryResult {
    connectors: plugin_state
      .connector_entries()
      .into_iter()
      .map(|connector| {
        let credential = plugin_state.connector_credential(&connector.connector_id);
        to_protocol_plugin_connector(connector, credential)
      })
      .collect(),
  }
}

pub(crate) fn build_protocol_hook_registry(
  plugins: &[PluginCatalogEntry],
) -> PluginHookRegistryResult {
  PluginHookRegistryResult {
    hooks: build_hook_registry(plugins)
      .into_iter()
      .map(to_protocol_plugin_hook)
      .collect(),
  }
}
