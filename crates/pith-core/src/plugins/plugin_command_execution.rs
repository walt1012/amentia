use std::collections::HashMap;

use pith_plugin_host::PluginCommandEntry as HostPluginCommandEntry;
use pith_protocol::TimelineItem;

use super::plugin_command_builtins::execute_builtin_plugin_command;
use super::plugin_command_types::{
  CompletedPluginCommandRun, PluginCommandOutput, PluginCommandSnapshot, PreparedPluginCommandRun,
};

pub fn execute_prepared_plugin_command_run(
  prepared: PreparedPluginCommandRun,
) -> CompletedPluginCommandRun {
  CompletedPluginCommandRun {
    request_id: prepared.request_id,
    output: execute_plugin_command_snapshot(prepared.snapshot),
  }
}

fn execute_plugin_command_snapshot(
  snapshot: PluginCommandSnapshot,
) -> std::result::Result<PluginCommandOutput, (i32, String)> {
  let builtin_result = execute_builtin_plugin_command(
    &snapshot.command,
    snapshot.workspace.as_ref(),
    snapshot.input.as_deref(),
    &snapshot.memory_notes,
  )?;

  let result_item = build_plugin_result_timeline_item(
    &snapshot.command,
    &builtin_result.execution_kind,
    builtin_result.content.clone(),
  );
  let assistant_item = TimelineItem {
    kind: "assistantMessage".to_string(),
    title: "Assistant".to_string(),
    content: format!(
      "{} completed through {}.\n\n{}",
      snapshot.command.title, snapshot.command.plugin_display_name, builtin_result.content
    ),
    attributes: Some(HashMap::from([
      ("pluginId".to_string(), snapshot.command.plugin_id.clone()),
      ("commandId".to_string(), snapshot.command.command_id.clone()),
      (
        "executionKind".to_string(),
        builtin_result.execution_kind.clone(),
      ),
    ])),
  };
  Ok(PluginCommandOutput {
    thread_id: snapshot.thread_id,
    command: snapshot.command,
    workspace: snapshot.workspace,
    input: snapshot.input,
    items: vec![snapshot.command_item, result_item, assistant_item],
  })
}

fn build_plugin_result_timeline_item(
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
