use std::collections::HashMap;

use pith_plugin_host::PluginCommandEntry as HostPluginCommandEntry;
use pith_protocol::{TimelineItem, WorkspaceSummary};

use super::plugin_command_types::PluginConnectorExecutionRef;
use crate::approval_types::PendingApproval;

pub(crate) const PLUGIN_COMMAND_APPROVAL_ACTION: &str = "run_plugin_command";
pub(crate) const PLUGIN_COMMAND_CONNECTOR_APPROVAL_REASON: &str =
  "Plugin actions require approval before launch.";

pub(super) fn plugin_command_requires_user_approval(
  command: &HostPluginCommandEntry,
  connector_refs: &[PluginConnectorExecutionRef],
) -> bool {
  command.execution.is_some()
    && connector_refs
      .iter()
      .any(PluginConnectorExecutionRef::requires_user_approval)
}

pub(super) fn build_plugin_command_approval_request(
  approval_id: String,
  thread_id: &str,
  command: &HostPluginCommandEntry,
  workspace: Option<&WorkspaceSummary>,
  input: Option<&str>,
  connector_refs: &[PluginConnectorExecutionRef],
) -> (PendingApproval, Vec<TimelineItem>) {
  let approval = PendingApproval {
    id: approval_id,
    thread_id: thread_id.to_string(),
    action: PLUGIN_COMMAND_APPROVAL_ACTION.to_string(),
    title: format!("Run {}", command.title),
    relative_path: format!("plugin:{}", command.plugin_id),
    content: input.map(str::to_string),
    command: Some(command.command_id.clone()),
  };
  let workspace_label = workspace
    .map(|workspace| workspace.display_name.as_str())
    .unwrap_or("No Project");
  let connector_ids = connector_refs
    .iter()
    .map(|connector| connector.connector_id.as_str())
    .collect::<Vec<_>>()
    .join(", ");
  let connector_services = connector_refs
    .iter()
    .map(|connector| connector.service.as_str())
    .collect::<Vec<_>>()
    .join(", ");
  let credential_stores = connector_refs
    .iter()
    .map(|connector| connector.credential_provider.store.as_str())
    .collect::<Vec<_>>()
    .join(", ");
  let credential_providers = connector_refs
    .iter()
    .map(|connector| connector.credential_provider.provider.as_str())
    .collect::<Vec<_>>()
    .join(", ");
  let credential_labels = connector_refs
    .iter()
    .map(|connector| connector.credential_provider.label.as_str())
    .collect::<Vec<_>>()
    .join(", ");
  let secret_bindings = connector_refs
    .iter()
    .map(PluginConnectorExecutionRef::credential_binding)
    .collect::<Vec<_>>()
    .join(", ");
  let mut approval_attributes = HashMap::from([
    ("approvalId".to_string(), approval.id.clone()),
    ("action".to_string(), approval.action.clone()),
    ("commandId".to_string(), command.command_id.clone()),
    ("pluginId".to_string(), command.plugin_id.clone()),
    (
      "pluginDisplayName".to_string(),
      command.plugin_display_name.clone(),
    ),
    ("sourcePath".to_string(), command.source_path.clone()),
    ("connectorIds".to_string(), connector_ids.clone()),
    ("connectorServices".to_string(), connector_services.clone()),
    (
      "connectorCredentialStores".to_string(),
      credential_stores.clone(),
    ),
    (
      "connectorCredentialProviders".to_string(),
      credential_providers.clone(),
    ),
    (
      "connectorCredentialLabels".to_string(),
      credential_labels.clone(),
    ),
    (
      "connectorSecretBindings".to_string(),
      secret_bindings.clone(),
    ),
  ]);
  if connector_refs.len() == 1 {
    approval_attributes.insert(
      "connectorId".to_string(),
      connector_refs[0].connector_id.clone(),
    );
  }
  if let Some(execution_kind) = command.execution_kind.as_ref() {
    approval_attributes.insert("executionKind".to_string(), execution_kind.clone());
  }
  if let Some(input) = input {
    approval_attributes.insert("commandInput".to_string(), input.to_string());
  }

  (
    approval.clone(),
    vec![
      TimelineItem {
        kind: "approvalRequested".to_string(),
        title: "Plugin Action Approval Requested".to_string(),
        content: format!(
          "Pith needs approval before running {} from {} in {}.\nConnections: {} ({})\nCredentials: {} | stores {} | bindings {}",
          command.title,
          command.plugin_display_name,
          workspace_label,
          connector_services,
          connector_ids,
          credential_labels,
          credential_stores,
          secret_bindings
        ),
        attributes: Some(approval_attributes),
      },
      TimelineItem {
        kind: "assistantMessage".to_string(),
        title: "Assistant".to_string(),
        content: "Pith is waiting for your approval before running this plugin action."
          .to_string(),
        attributes: Some(HashMap::from([
          ("approvalId".to_string(), approval.id.clone()),
          ("commandId".to_string(), command.command_id.clone()),
          ("pluginId".to_string(), command.plugin_id.clone()),
        ])),
      },
    ],
  )
}
