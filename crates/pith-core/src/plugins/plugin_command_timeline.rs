use std::collections::HashMap;

use pith_plugin_host::PluginCommandEntry as HostPluginCommandEntry;
use pith_protocol::{TimelineItem, WorkspaceSummary};

use crate::context_memory_pack::{merge_memory_context_attributes, MemoryContextPack};

const PLUGIN_FAILURE_LOG_PREVIEW_LIMIT: usize = 2048;

pub(super) fn build_plugin_command_timeline_item(
  command: &HostPluginCommandEntry,
  workspace: Option<&WorkspaceSummary>,
  input: Option<&str>,
  memory_context: &MemoryContextPack,
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
  attributes.extend(HashMap::from([
    ("pluginId".to_string(), command.plugin_id.clone()),
    ("commandId".to_string(), command.command_id.clone()),
    ("pluginCommandStatus".to_string(), command_status.to_string()),
    ("pluginRunnerErrorCode".to_string(), code.to_string()),
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
