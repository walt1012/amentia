use std::collections::HashMap;

use pith_plugin_host::PluginCommandEntry as HostPluginCommandEntry;
use pith_protocol::{TimelineItem, WorkspaceSummary};

use super::plugin_command_types::PluginConnectorExecutionRef;
use crate::context_memory_pack::{merge_memory_context_attributes, MemoryContextPack};

const PLUGIN_FAILURE_LOG_PREVIEW_LIMIT: usize = 2048;

pub(super) fn build_plugin_command_timeline_item(
  command: &HostPluginCommandEntry,
  workspace: Option<&WorkspaceSummary>,
  input: Option<&str>,
  memory_context: &MemoryContextPack,
  connector_refs: &[PluginConnectorExecutionRef],
) -> TimelineItem {
  let mut attributes = HashMap::from([
    ("commandId".to_string(), command.command_id.clone()),
    ("pluginId".to_string(), command.plugin_id.clone()),
    (
      "pluginDisplayName".to_string(),
      command.plugin_display_name.clone(),
    ),
    ("sourcePath".to_string(), command.source_path.clone()),
  ]);
  if let Some(workspace) = workspace {
    attributes.insert(
      "workspaceDisplayName".to_string(),
      workspace.display_name.clone(),
    );
  }
  if let Some(input) = input {
    attributes.insert("commandInput".to_string(), input.to_string());
  }
  if let Some(execution_kind) = command.execution_kind.as_ref() {
    attributes.insert("executionKind".to_string(), execution_kind.clone());
  }
  insert_connector_context_attributes(&mut attributes, connector_refs);
  merge_memory_context_attributes(&mut attributes, memory_context);

  let workspace_label = workspace
    .map(|entry| entry.display_name.clone())
    .unwrap_or_else(|| "No Workspace".to_string());
  let mut content = format!(
    "Run {} from {} in {}.\n{}",
    command.title, command.plugin_display_name, workspace_label, command.description
  );
  if let Some(input) = input {
    content.push_str(&format!("\nCommand input: {input}"));
  }

  TimelineItem {
    kind: "pluginCommand".to_string(),
    title: command.title.clone(),
    content,
    attributes: Some(attributes),
  }
}

fn insert_connector_context_attributes(
  attributes: &mut HashMap<String, String>,
  connector_refs: &[PluginConnectorExecutionRef],
) {
  if connector_refs.is_empty() {
    return;
  }

  attributes.insert(
    "connectorIds".to_string(),
    connector_refs
      .iter()
      .map(|connector| connector.connector_id.as_str())
      .collect::<Vec<_>>()
      .join(", "),
  );
  attributes.insert(
    "connectorCredentialStores".to_string(),
    connector_refs
      .iter()
      .map(|connector| connector.credential_provider.store.as_str())
      .collect::<Vec<_>>()
      .join(", "),
  );
  attributes.insert(
    "connectorCredentialProviders".to_string(),
    connector_refs
      .iter()
      .map(|connector| connector.credential_provider.provider.as_str())
      .collect::<Vec<_>>()
      .join(", "),
  );
  attributes.insert(
    "connectorCredentialHandles".to_string(),
    connector_refs
      .iter()
      .map(|connector| connector.credential_provider.handle.as_str())
      .collect::<Vec<_>>()
      .join(", "),
  );
  attributes.insert(
    "connectorCredentialLabels".to_string(),
    connector_refs
      .iter()
      .map(|connector| connector.credential_provider.label.as_str())
      .collect::<Vec<_>>()
      .join(", "),
  );
  attributes.insert(
    "connectorCredentialAuthorizedAt".to_string(),
    connector_refs
      .iter()
      .map(|connector| connector.credential_provider.authorized_at.to_string())
      .collect::<Vec<_>>()
      .join(", "),
  );
  attributes.insert(
    "connectorSecretBindings".to_string(),
    connector_refs
      .iter()
      .map(PluginConnectorExecutionRef::credential_binding)
      .collect::<Vec<_>>()
      .join(", "),
  );
  attributes.insert(
    "connectorServices".to_string(),
    connector_refs
      .iter()
      .map(|connector| connector.service.as_str())
      .collect::<Vec<_>>()
      .join(", "),
  );
}

pub(super) fn build_plugin_result_timeline_item(
  command: &HostPluginCommandEntry,
  execution_kind: &str,
  content: String,
) -> TimelineItem {
  TimelineItem {
    kind: "pluginResult".to_string(),
    title: format!("{} Result", command.title),
    content,
    attributes: Some(HashMap::from([
      ("pluginId".to_string(), command.plugin_id.clone()),
      ("commandId".to_string(), command.command_id.clone()),
      ("executionKind".to_string(), execution_kind.to_string()),
      ("sourcePath".to_string(), command.source_path.clone()),
    ])),
  }
}

pub(super) fn build_plugin_assistant_timeline_item(
  command: &HostPluginCommandEntry,
  execution_kind: &str,
  content: &str,
) -> TimelineItem {
  TimelineItem {
    kind: "assistantMessage".to_string(),
    title: "Assistant".to_string(),
    content: format!(
      "{} completed through {}.\n\n{}",
      command.title, command.plugin_display_name, content
    ),
    attributes: Some(HashMap::from([
      ("pluginId".to_string(), command.plugin_id.clone()),
      ("commandId".to_string(), command.command_id.clone()),
      ("executionKind".to_string(), execution_kind.to_string()),
    ])),
  }
}

pub(super) fn build_plugin_failure_timeline_item(
  command: &HostPluginCommandEntry,
  execution_kind: Option<&str>,
  code: i32,
  message: String,
  stdout: &str,
  stderr: &str,
  mut attributes: HashMap<String, String>,
) -> TimelineItem {
  let command_status = if code == -32055 {
    "cancelled"
  } else {
    "failed"
  };
  let title_status = if command_status == "cancelled" {
    "Cancelled"
  } else {
    "Failed"
  };
  let failure_kind = plugin_runner_failure_kind(code, &attributes);
  let recovery_hint = plugin_runner_recovery_hint(failure_kind);
  attributes.extend(HashMap::from([
    ("pluginId".to_string(), command.plugin_id.clone()),
    ("commandId".to_string(), command.command_id.clone()),
    (
      "pluginCommandStatus".to_string(),
      command_status.to_string(),
    ),
    ("pluginRunnerErrorCode".to_string(), code.to_string()),
    (
      "pluginRunnerFailureKind".to_string(),
      failure_kind.to_string(),
    ),
    (
      "pluginRunnerRecoveryHint".to_string(),
      recovery_hint.to_string(),
    ),
    ("sourcePath".to_string(), command.source_path.clone()),
  ]));
  if let Some(execution_kind) = execution_kind {
    attributes.insert("executionKind".to_string(), execution_kind.to_string());
  }

  let mut content = format!("{message}\n\nError code: {code}");
  if !stderr.trim().is_empty() {
    content.push_str("\n\nStderr:\n");
    content.push_str(&bounded_failure_log_preview(stderr));
  }
  if !stdout.trim().is_empty() {
    content.push_str("\n\nStdout:\n");
    content.push_str(&bounded_failure_log_preview(stdout));
  }

  TimelineItem {
    kind: "warning".to_string(),
    title: format!("{} {title_status}", command.title),
    content,
    attributes: Some(attributes),
  }
}

fn bounded_failure_log_preview(content: &str) -> String {
  let trimmed = content.trim();
  let mut preview = trimmed
    .chars()
    .take(PLUGIN_FAILURE_LOG_PREVIEW_LIMIT)
    .collect::<String>();
  if trimmed.chars().count() > PLUGIN_FAILURE_LOG_PREVIEW_LIMIT {
    preview.push_str("\n[truncated]");
  }
  preview
}

fn plugin_runner_failure_kind(code: i32, attributes: &HashMap<String, String>) -> &'static str {
  if code == -32055 {
    return "cancelled";
  }
  if code == -32056 {
    return "timeout";
  }
  if attributes
    .get("pluginRunnerSetupStatus")
    .is_some_and(|status| status == "failed")
  {
    return "runnerSetup";
  }
  if code == -32053 {
    return "unsupportedExecution";
  }
  if attributes.contains_key("mcpProtocolStatus") {
    return "mcpProtocol";
  }
  if attributes.contains_key("pluginRunnerOutputStatus") {
    return "outputContract";
  }
  if attributes.contains_key("pluginRunnerExitReason") {
    return "processExit";
  }
  "runnerSetup"
}

fn plugin_runner_recovery_hint(failure_kind: &str) -> &'static str {
  match failure_kind {
    "cancelled" => "Run the command again when the current task is ready.",
    "timeout" => "Check whether the runner is waiting for input or doing unbounded work.",
    "unsupportedExecution" => {
      "Update the plugin command manifest to declare a supported execution contract."
    }
    "mcpProtocol" => "Check the MCP server command and stdout JSON-RPC framing.",
    "outputContract" => {
      "Return plain text, a valid JSON output envelope, valid timeline items, or memory notes."
    }
    "processExit" => "Inspect runner stderr, stdout, and exit status.",
    _ => "Check the plugin manifest, entrypoint path, sandbox, and local files.",
  }
}

#[cfg(test)]
mod tests {
  use std::collections::HashMap;

  use super::*;

  #[test]
  fn failure_kind_keeps_unsupported_execution_without_setup_marker() {
    assert_eq!(
      plugin_runner_failure_kind(-32053, &HashMap::new()),
      "unsupportedExecution"
    );
  }

  #[test]
  fn failure_kind_prefers_runner_setup_marker_over_unsupported_code() {
    let attributes = HashMap::from([(
      "pluginRunnerSetupStatus".to_string(),
      "failed".to_string(),
    )]);

    assert_eq!(plugin_runner_failure_kind(-32053, &attributes), "runnerSetup");
  }
}
