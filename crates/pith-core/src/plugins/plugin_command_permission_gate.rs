use std::collections::HashMap;

use pith_plugin_host::PluginCommandEntry as HostPluginCommandEntry;
use pith_protocol::{TimelineItem, WorkspaceSummary};

use super::plugin_command_types::PluginConnectorExecutionRef;
use super::plugin_permissions::build_permission_denied_items;

pub(crate) fn plugin_command_permission_blocker(
  command: &HostPluginCommandEntry,
  connector_backed: bool,
) -> Option<String> {
  let execution = command.execution.as_ref()?;
  if execution.driver != "mcp" {
    return None;
  }

  if !command_declares_permission(command, "mcp.connect") {
    return Some(plugin_command_permission_blocker_message(
      command,
      "mcp.connect",
      "run an MCP command",
    ));
  }

  if connector_backed && !command_declares_permission(command, "network.outbound") {
    return Some(plugin_command_permission_blocker_message(
      command,
      "network.outbound",
      "run a connector-backed MCP command",
    ));
  }

  None
}

pub(super) fn plugin_command_permission_denied_items(
  command: &HostPluginCommandEntry,
  workspace: Option<&WorkspaceSummary>,
  connector_refs: &[PluginConnectorExecutionRef],
) -> Option<Vec<TimelineItem>> {
  let execution = command.execution.as_ref()?;
  if execution.driver != "mcp" {
    return None;
  }

  if !command_declares_permission(command, "mcp.connect") {
    return Some(build_plugin_command_denial(
      command,
      workspace,
      "mcp.connect",
      "run an MCP command",
    ));
  }

  if !connector_refs.is_empty() && !command_declares_permission(command, "network.outbound") {
    return Some(build_plugin_command_denial(
      command,
      None,
      "network.outbound",
      "run a connector-backed MCP command",
    ));
  }

  None
}

fn command_declares_permission(command: &HostPluginCommandEntry, permission: &str) -> bool {
  command
    .permissions
    .iter()
    .any(|declared_permission| declared_permission == permission)
}

fn plugin_command_permission_blocker_message(
  command: &HostPluginCommandEntry,
  permission: &str,
  blocked_action: &str,
) -> String {
  format!(
    "Plugin command `{}` needs `{}` permission to {}.",
    command.command_id, permission, blocked_action
  )
}

fn build_plugin_command_denial(
  command: &HostPluginCommandEntry,
  workspace: Option<&WorkspaceSummary>,
  permission: &str,
  blocked_action: &str,
) -> Vec<TimelineItem> {
  let mut permission_sources = HashMap::new();
  for declared_permission in &command.permissions {
    permission_sources
      .entry(declared_permission.clone())
      .or_insert_with(Vec::new)
      .push(command.plugin_display_name.clone());
  }

  build_permission_denied_items(
    &permission_sources,
    permission,
    blocked_action,
    workspace
      .map(|workspace| workspace.display_name.as_str())
      .unwrap_or("the web"),
    HashMap::from([
      ("commandId".to_string(), command.command_id.clone()),
      ("pluginId".to_string(), command.plugin_id.clone()),
      (
        "pluginDisplayName".to_string(),
        command.plugin_display_name.clone(),
      ),
      (
        "permissionGate".to_string(),
        "pluginCommandExecution".to_string(),
      ),
      ("sourcePath".to_string(), command.source_path.clone()),
    ]),
  )
}
