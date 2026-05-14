use pith_protocol::{JsonRpcResponse, TurnStartResult};

use super::plugin_command_memory::capture_plugin_command_output_memory;
use super::plugin_command_types::{CompletedPluginCommandRun, PluginCommandOutput};
use crate::approval_state::approvals_for_thread;
use crate::thread_summary::refresh_thread_summary_note;
use crate::RuntimeContext;

pub fn complete_prepared_plugin_command_run(
  context: &mut RuntimeContext,
  completed: CompletedPluginCommandRun,
) -> JsonRpcResponse {
  context
    .execution_state
    .remove_running_plugin_command(&completed.running_id);
  match completed.output {
    Ok(output) => match complete_plugin_command_items(context, output) {
      Ok(result) => JsonRpcResponse::success(completed.request_id, &result),
      Err((code, message)) => JsonRpcResponse::error(completed.request_id, code, message),
    },
    Err((code, message)) => JsonRpcResponse::error(completed.request_id, code, message),
  }
}

fn complete_plugin_command_items(
  context: &mut RuntimeContext,
  output: PluginCommandOutput,
) -> std::result::Result<TurnStartResult, (i32, String)> {
  let PluginCommandOutput {
    thread_id: requested_thread_id,
    command,
    workspace,
    input,
    mut items,
    capture_memory,
    runner_memory_notes,
    pending_approval,
  } = output;
  if let Some(approval) = pending_approval {
    context.execution_state.insert_pending_approval(approval);
  }
  let prepared_thread = {
    let Some(thread) = context.thread_state.find_mut(&requested_thread_id) else {
      return Err((-32004, "Thread not found".to_string()));
    };

    let prepared_thread = thread.begin_plugin_command(workspace.clone());
    thread.append_items(items.clone());
    thread.mark_ready();
    prepared_thread
  };
  let thread_id = prepared_thread.thread_id;
  let turn_id = prepared_thread.turn_id;

  context
    .persist_runtime_state()
    .map_err(|error| (-32010, error.to_string()))?;
  refresh_thread_summary_note(context, &thread_id).map_err(|error| (-32012, error.to_string()))?;

  let memory_items = capture_plugin_command_output_memory(
    context,
    &thread_id,
    &command,
    workspace.as_ref(),
    input.as_deref(),
    &items,
    capture_memory,
    &runner_memory_notes,
  );
  if !memory_items.is_empty() {
    if let Some(thread) = context.thread_state.find_mut(&thread_id) {
      thread.append_items(memory_items.clone());
    }
    items.extend(memory_items);
    context
      .persist_runtime_state()
      .map_err(|error| (-32010, error.to_string()))?;
    refresh_thread_summary_note(context, &thread_id).map_err(|error| (-32012, error.to_string()))?;
  }

  Ok(TurnStartResult {
    turn_id,
    thread_id,
    items,
    pending_approvals: approvals_for_thread(context, &requested_thread_id),
    active_turn_id: None,
  })
}
