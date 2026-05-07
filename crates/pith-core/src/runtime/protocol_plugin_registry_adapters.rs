use std::collections::HashMap;

use pith_plugin_host::{
  build_capability_registry, build_command_registry, build_connector_registry, build_hook_registry,
  PluginCapabilityRegistration as HostPluginCapabilityRegistration, PluginCatalogEntry,
  PluginCommandEntry as HostPluginCommandEntry, PluginConnectorEntry as HostPluginConnectorEntry,
  PluginHookEntry as HostPluginHookEntry,
};
use pith_protocol::{
  PluginCapabilityRegistration, PluginCapabilityRegistryResult, PluginCapabilityRegistrySummary,
  PluginCommandRegistryResult, PluginCommandSummary, PluginConnectorRegistryResult,
  PluginConnectorSummary, PluginHookRegistryResult, PluginHookSummary,
};

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
  plugins: &[PluginCatalogEntry],
) -> PluginConnectorRegistryResult {
  PluginConnectorRegistryResult {
    connectors: build_connector_registry(plugins)
      .into_iter()
      .map(to_protocol_plugin_connector)
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

fn to_protocol_capability(
  capability: HostPluginCapabilityRegistration,
) -> PluginCapabilityRegistration {
  PluginCapabilityRegistration {
    capability_id: capability.capability_id,
    kind: capability.kind,
    identifier: capability.identifier,
    plugin_id: capability.plugin_id,
    plugin_display_name: capability.plugin_display_name,
    permissions: capability.permissions,
    manifest_path: capability.manifest_path,
    metadata: capability.metadata,
  }
}

fn to_protocol_plugin_command(command: HostPluginCommandEntry) -> PluginCommandSummary {
  let memory_summary = command
    .memory_note_title
    .as_ref()
    .map(|title| format!("Stores a workspace memory note as `{title}` after execution."));
  PluginCommandSummary {
    command_id: command.command_id,
    title: command.title,
    description: command.description,
    plugin_id: command.plugin_id,
    plugin_display_name: command.plugin_display_name,
    permissions: command.permissions,
    source_path: command.source_path,
    execution_kind: command.execution_kind,
    memory_summary,
  }
}

fn to_protocol_plugin_connector(connector: HostPluginConnectorEntry) -> PluginConnectorSummary {
  PluginConnectorSummary {
    connector_id: connector.connector_id,
    display_name: connector.display_name,
    service: connector.service,
    plugin_id: connector.plugin_id,
    plugin_display_name: connector.plugin_display_name,
    enabled: connector.enabled,
    status: connector.status,
    permissions: connector.permissions,
    manifest_path: connector.manifest_path,
    homepage: connector.homepage,
    auth_type: connector.auth_type,
    auth_required: connector.auth_required,
    auth_scopes: connector.auth_scopes,
    credential_store: connector.credential_store,
  }
}

fn to_protocol_plugin_hook(hook: HostPluginHookEntry) -> PluginHookSummary {
  let memory_summary = hook
    .memory_note_title
    .as_ref()
    .map(|title| format!("Stores a workspace memory note as `{title}` when the hook runs."));
  PluginHookSummary {
    hook_id: hook.hook_id,
    title: hook.title,
    description: hook.description,
    event: hook.event,
    plugin_id: hook.plugin_id,
    plugin_display_name: hook.plugin_display_name,
    permissions: hook.permissions,
    source_path: hook.source_path,
    memory_summary,
  }
}
