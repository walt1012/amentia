use super::plugin_command_builtins::execute_builtin_plugin_command;
use super::plugin_command_timeline::{
  build_plugin_assistant_timeline_item, build_plugin_result_timeline_item,
};
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
  let assistant_item = build_plugin_assistant_timeline_item(
    &snapshot.command,
    &builtin_result.execution_kind,
    &builtin_result.content,
  );
  Ok(PluginCommandOutput {
    thread_id: snapshot.thread_id,
    command: snapshot.command,
    workspace: snapshot.workspace,
    input: snapshot.input,
    items: vec![snapshot.command_item, result_item, assistant_item],
  })
}
