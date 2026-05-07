use pith_plugin_host::{
  PluginCapabilityRegistration as HostPluginCapabilityRegistration,
  PluginCommandEntry as HostPluginCommandEntry, PluginConnectorEntry as HostPluginConnectorEntry,
  PluginHookEntry as HostPluginHookEntry,
};
use pith_protocol::{
  PluginCapabilityRegistration, PluginCommandSummary, PluginConnectorSummary, PluginHookSummary,
};

pub(super) fn to_protocol_capability(
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

pub(super) fn to_protocol_plugin_command(command: HostPluginCommandEntry) -> PluginCommandSummary {
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

pub(super) fn to_protocol_plugin_connector(
  connector: HostPluginConnectorEntry,
) -> PluginConnectorSummary {
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

pub(super) fn to_protocol_plugin_hook(hook: HostPluginHookEntry) -> PluginHookSummary {
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
