use std::collections::HashMap;

use pith_plugin_host::PluginCommandEntry as HostPluginCommandEntry;
use pith_protocol::{TimelineItem, WorkspaceSummary};

use super::plugin_command_recovery_hints::readiness_repair_hint;
use super::plugin_command_types::PluginConnectorExecutionRef;
use super::plugin_permissions::build_permission_denied_items;

const MCP_STDIO_PLUGIN_ACTION: &str = "run an MCP stdio plugin command";
const CONNECTOR_PLUGIN_ACTION: &str = "run a connector-backed plugin command";

pub(crate) fn plugin_command_permission_blocker(
  command: &HostPluginCommandEntry,
  connector_backed: bool,
) -> Option<String> {
  let execution = command.execution.as_ref()?;

  if execution.driver == "mcp" && !command_declares_permission(command, "mcp.connect") {
    return Some(plugin_command_permission_blocker_message(
      command,
      "mcp.connect",
      MCP_STDIO_PLUGIN_ACTION,
    ));
  }

  if connector_backed && !command_declares_permission(command, "network.outbound") {
    return Some(plugin_command_permission_blocker_message(
      command,
      "network.outbound",
      CONNECTOR_PLUGIN_ACTION,
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

  if execution.driver == "mcp" && !command_declares_permission(command, "mcp.connect") {
    return Some(build_plugin_command_denial(
      command,
      workspace,
      "mcp.connect",
      MCP_STDIO_PLUGIN_ACTION,
    ));
  }

  if !connector_refs.is_empty() && !command_declares_permission(command, "network.outbound") {
    return Some(build_plugin_command_denial(
      command,
      None,
      "network.outbound",
      CONNECTOR_PLUGIN_ACTION,
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
  let run_blocker = plugin_command_permission_blocker_message(command, permission, blocked_action);
  let run_repair_hint = readiness_repair_hint("missingPermission", &run_blocker);
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
      ("pluginCommandStatus".to_string(), "blocked".to_string()),
      ("runStatus".to_string(), "missingPermission".to_string()),
      ("runBlocker".to_string(), run_blocker),
      ("runRepairHint".to_string(), run_repair_hint),
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

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn command_permission_denial_uses_blocked_command_metadata() {
    let command = HostPluginCommandEntry {
      command_id: "test-plugin::run".to_string(),
      title: "Run Test Plugin".to_string(),
      description: "Run a test plugin command.".to_string(),
      prompt: "Run the plugin.".to_string(),
      plugin_id: "test-plugin".to_string(),
      plugin_display_name: "Test Plugin".to_string(),
      permissions: vec!["mcp.connect".to_string()],
      source_path: "plugins/test-plugin/commands/run.json".to_string(),
      execution: None,
      execution_kind: Some("mcp.test".to_string()),
      manifest_error: None,
      memory_note_title: None,
      memory_note_source: None,
      memory_note_tags: vec![],
    };

    let items = build_plugin_command_denial(
      &command,
      None,
      "network.outbound",
      "run a connector-backed plugin command",
    );
    let attributes = items[0].attributes.as_ref().expect("attributes");

    assert_eq!(
      attributes.get("pluginCommandStatus").map(String::as_str),
      Some("blocked")
    );
    assert_eq!(
      attributes.get("runStatus").map(String::as_str),
      Some("missingPermission")
    );
    assert_eq!(
      attributes.get("sourcePath").map(String::as_str),
      Some("plugins/test-plugin/commands/run.json")
    );
    assert!(attributes
      .get("runRepairHint")
      .expect("repair hint")
      .contains("required permission"));
  }

  #[test]
  fn command_permission_blockers_use_precise_plugin_actions() {
    let empty_input = pith_plugin_host::PluginCommandEnvelopeEntry {
      envelope: "pith.plugin.command.input".to_string(),
      fields: vec![],
    };
    let empty_output = pith_plugin_host::PluginCommandEnvelopeEntry {
      envelope: "pith.plugin.command.output".to_string(),
      fields: vec![],
    };
    let mcp_command = HostPluginCommandEntry {
      command_id: "test-plugin::mcp-run".to_string(),
      title: "Run MCP Plugin".to_string(),
      description: "Run an MCP stdio plugin command.".to_string(),
      prompt: "Run the plugin.".to_string(),
      plugin_id: "test-plugin".to_string(),
      plugin_display_name: "Test Plugin".to_string(),
      permissions: vec![],
      source_path: "plugins/test-plugin/commands/mcp-run.json".to_string(),
      execution: Some(pith_plugin_host::PluginCommandExecutionEntry {
        kind: "mcp.test".to_string(),
        driver: "mcp".to_string(),
        entrypoint: Some("local.run".to_string()),
        connector_ids: None,
        workflow_id: None,
        workflow: None,
        input: empty_input.clone(),
        output: empty_output.clone(),
      }),
      execution_kind: Some("mcp.test".to_string()),
      manifest_error: None,
      memory_note_title: None,
      memory_note_source: None,
      memory_note_tags: vec![],
    };
    let connector_command = HostPluginCommandEntry {
      command_id: "test-plugin::connector-run".to_string(),
      title: "Run Connector Plugin".to_string(),
      description: "Run a connector-backed plugin command.".to_string(),
      prompt: "Run the plugin.".to_string(),
      plugin_id: "test-plugin".to_string(),
      plugin_display_name: "Test Plugin".to_string(),
      permissions: vec!["mcp.connect".to_string()],
      source_path: "plugins/test-plugin/commands/connector-run.json".to_string(),
      execution: Some(pith_plugin_host::PluginCommandExecutionEntry {
        kind: "stdio.test".to_string(),
        driver: "stdio".to_string(),
        entrypoint: Some("runner.sh".to_string()),
        connector_ids: Some(vec!["notion".to_string()]),
        workflow_id: Some("notion.create-page".to_string()),
        workflow: None,
        input: empty_input,
        output: empty_output,
      }),
      execution_kind: Some("stdio.test".to_string()),
      manifest_error: None,
      memory_note_title: None,
      memory_note_source: None,
      memory_note_tags: vec![],
    };

    let mcp_blocker = plugin_command_permission_blocker(&mcp_command, false).expect("mcp blocker");
    let connector_blocker =
      plugin_command_permission_blocker(&connector_command, true).expect("connector blocker");

    assert!(mcp_blocker.contains("MCP stdio plugin command"));
    assert!(connector_blocker.contains("connector-backed plugin command"));
    assert!(!connector_blocker.contains("MCP"));
  }
}
