use std::collections::HashMap;

use pith_plugin_host::PluginCommandEntry as HostPluginCommandEntry;

use super::plugin_command_builtins::execute_builtin_plugin_command;
use super::plugin_command_builtins::is_supported_builtin_execution;
use super::plugin_command_runner::{
  is_supported_external_plugin_execution, run_external_plugin_command,
};
use super::plugin_command_timeline::{
  build_plugin_assistant_timeline_item, build_plugin_result_timeline_item,
};
use super::plugin_command_types::{
  CompletedPluginCommandRun, PluginCommandOutput, PluginCommandSnapshot, PreparedPluginCommandRun,
};

pub fn execute_prepared_plugin_command_run(
  prepared: PreparedPluginCommandRun,
) -> CompletedPluginCommandRun {
  let running_id = prepared.snapshot.running_id.clone();
  CompletedPluginCommandRun {
    request_id: prepared.request_id,
    running_id,
    output: execute_plugin_command_snapshot(prepared.snapshot),
  }
}

pub(crate) fn is_supported_plugin_command_execution(command: &HostPluginCommandEntry) -> bool {
  is_supported_builtin_execution(command.execution_kind.as_deref())
    || is_supported_external_plugin_execution(command)
}

fn execute_plugin_command_snapshot(
  snapshot: PluginCommandSnapshot,
) -> std::result::Result<PluginCommandOutput, (i32, String)> {
  let (execution_kind, content, attributes) =
    if is_supported_builtin_execution(snapshot.command.execution_kind.as_deref()) {
      let builtin_result = execute_builtin_plugin_command(
        &snapshot.command,
        snapshot.workspace.as_ref(),
        snapshot.input.as_deref(),
        &snapshot.memory_notes,
      )?;
      (
        builtin_result.execution_kind,
        builtin_result.content,
        HashMap::new(),
      )
    } else {
      let runner_result = run_external_plugin_command(
        &snapshot.command,
        &snapshot.thread_id,
        snapshot.workspace.as_ref(),
        snapshot.input.as_deref(),
        &snapshot.cancellation,
      )?;
      if !runner_result.items.is_empty() {
        let mut items = vec![snapshot.command_item];
        items.extend(runner_result.items);
        return Ok(PluginCommandOutput {
          thread_id: snapshot.thread_id,
          command: snapshot.command,
          workspace: snapshot.workspace,
          input: snapshot.input,
          items,
        });
      }
      (
        runner_result.execution_kind,
        runner_result.content,
        runner_result.attributes,
      )
    };

  let mut result_item =
    build_plugin_result_timeline_item(&snapshot.command, &execution_kind, content.clone());
  if let Some(item_attributes) = result_item.attributes.as_mut() {
    item_attributes.extend(attributes);
  }
  let assistant_item =
    build_plugin_assistant_timeline_item(&snapshot.command, &execution_kind, &content);
  Ok(PluginCommandOutput {
    thread_id: snapshot.thread_id,
    command: snapshot.command,
    workspace: snapshot.workspace,
    input: snapshot.input,
    items: vec![snapshot.command_item, result_item, assistant_item],
  })
}
