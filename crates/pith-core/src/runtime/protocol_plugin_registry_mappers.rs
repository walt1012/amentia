use pith_plugin_host::{
  PluginCapabilityRegistration as HostPluginCapabilityRegistration,
  PluginCommandEntry as HostPluginCommandEntry,
  PluginCommandEnvelopeEntry as HostPluginCommandEnvelopeEntry,
  PluginCommandEnvelopeFieldEntry as HostPluginCommandEnvelopeFieldEntry,
  PluginConnectorEntry as HostPluginConnectorEntry, PluginHookEntry as HostPluginHookEntry,
};
use pith_protocol::{
  PluginCapabilityRegistration, PluginCommandEnvelopeFieldSummary, PluginCommandEnvelopeSummary,
  PluginCommandExecutionSummary, PluginCommandSummary, PluginConnectorSummary, PluginHookSummary,
};

use crate::plugins::plugin_command_execution::is_supported_plugin_command_execution;
use crate::plugins::plugin_command_readiness::PluginCommandReadiness;
use crate::runtime_plugins::PluginConnectorCredentialState;

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

pub(super) fn to_protocol_plugin_command(
  command: HostPluginCommandEntry,
  readiness: PluginCommandReadiness,
) -> PluginCommandSummary {
  let memory_summary = command
    .memory_note_title
    .as_ref()
    .map(|title| format!("Stores a workspace memory note as `{title}` after execution."));
  let supported = is_supported_plugin_command_execution(&command);
  let execution = command
    .execution
    .as_ref()
    .map(|execution| PluginCommandExecutionSummary {
      kind: execution.kind.clone(),
      driver: execution.driver.clone(),
      entrypoint: execution.entrypoint.clone(),
      input: to_protocol_plugin_command_envelope(&execution.input),
      output: to_protocol_plugin_command_envelope(&execution.output),
      supported,
    });
  PluginCommandSummary {
    command_id: command.command_id,
    title: command.title,
    description: command.description,
    plugin_id: command.plugin_id,
    plugin_display_name: command.plugin_display_name,
    permissions: command.permissions,
    source_path: command.source_path,
    execution,
    execution_kind: command.execution_kind,
    memory_summary,
    run_status: readiness.run_status,
    run_blocker: readiness.run_blocker,
    required_connector_ids: readiness.required_connector_ids,
  }
}

fn to_protocol_plugin_command_envelope(
  envelope: &HostPluginCommandEnvelopeEntry,
) -> PluginCommandEnvelopeSummary {
  PluginCommandEnvelopeSummary {
    envelope: envelope.envelope.clone(),
    fields: envelope
      .fields
      .iter()
      .map(to_protocol_plugin_command_envelope_field)
      .collect(),
  }
}

fn to_protocol_plugin_command_envelope_field(
  field: &HostPluginCommandEnvelopeFieldEntry,
) -> PluginCommandEnvelopeFieldSummary {
  PluginCommandEnvelopeFieldSummary {
    name: field.name.clone(),
    kind: field.kind.clone(),
    required: field.required,
    description: field.description.clone(),
  }
}

pub(super) fn to_protocol_plugin_connector(
  connector: HostPluginConnectorEntry,
  credential: Option<&PluginConnectorCredentialState>,
) -> PluginConnectorSummary {
  let credential_present = credential.is_some();
  let auth_status = connector_auth_status(&connector, credential_present);
  let status = connector_status(&connector, credential_present);
  PluginConnectorSummary {
    connector_id: connector.connector_id,
    display_name: connector.display_name,
    service: connector.service,
    plugin_id: connector.plugin_id,
    plugin_display_name: connector.plugin_display_name,
    enabled: connector.enabled,
    status,
    permissions: connector.permissions,
    manifest_path: connector.manifest_path,
    homepage: connector.homepage,
    auth_type: connector.auth_type,
    auth_required: connector.auth_required,
    auth_scopes: connector.auth_scopes,
    credential_store: connector.credential_store,
    auth_status,
    credential_present,
    credential_label: credential.map(|state| state.credential_label.clone()),
    authorized_at: credential.map(|state| state.authorized_at),
  }
}

fn connector_status(connector: &HostPluginConnectorEntry, credential_present: bool) -> String {
  if !connector.enabled {
    return "disabled".to_string();
  }
  if connector.auth_required && !credential_present {
    return "needsAuth".to_string();
  }
  "ready".to_string()
}

fn connector_auth_status(connector: &HostPluginConnectorEntry, credential_present: bool) -> String {
  if !connector.enabled {
    return "disabled".to_string();
  }
  if !connector.auth_required {
    return "notRequired".to_string();
  }
  if credential_present {
    return "authorized".to_string();
  }
  "needsAuth".to_string()
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
