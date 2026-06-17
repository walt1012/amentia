use std::collections::HashMap;
use std::panic::{catch_unwind, AssertUnwindSafe};

use amentia_model_runtime::GenerationCancellation;
use amentia_protocol::{JsonRpcRequest, JsonRpcResponse, TurnStartParams, TurnStartResult};

use crate::approval_state::approvals_for_thread;
use crate::plugin_commands::capture_plugin_command_output_memory;
use crate::plugin_permissions::granted_permission_sources;
use crate::request_params::parse_required_params;
use crate::request_state::{CompletedTurnStart, PreparedTurnSnapshot, PreparedTurnStart};
use crate::runtime_context::RuntimeContext;
use crate::thread_summary::refresh_thread_summary_note;
use crate::turn::local_execution_safety::LocalExecutionSafetyMode;
use crate::turn::turn_agent_loop::LOOP_MAX_STEPS;
use crate::turn_actions;

pub(crate) fn handle_turn_start(
  context: &mut RuntimeContext,
  request: JsonRpcRequest,
) -> JsonRpcResponse {
  let prepared = match prepare_turn_start(context, request) {
    Ok(prepared) => prepared,
    Err(response) => return response,
  };
  let completed = execute_prepared_turn_start(prepared);
  complete_prepared_turn_start(context, completed)
}

pub fn prepare_turn_start(
  context: &mut RuntimeContext,
  request: JsonRpcRequest,
) -> std::result::Result<PreparedTurnStart, JsonRpcResponse> {
  let params = parse_required_params::<TurnStartParams>(&request, "turn/start")?;

  if let Err(message) = ensure_turn_model_ready(context) {
    return Err(JsonRpcResponse::error(request.id, -32060, message));
  }

  let current_workspace = context.workspace_state.current_cloned();
  let model_runtime = context.model_state.snapshot();
  let cancellation = GenerationCancellation::new();
  let memory_notes = context.memory_state.snapshot_notes();
  let plugin_state = context.plugin_state.clone();
  let permission_sources = granted_permission_sources(context.plugin_state.catalog());
  let local_execution_safety_mode =
    LocalExecutionSafetyMode::from_request(params.local_execution_safety_mode.as_deref());
  let prepared_thread = {
    let Some(thread) = context.thread_state.find_mut(&params.thread_id) else {
      return Err(JsonRpcResponse::error(
        request.id,
        -32004,
        "Session not found",
      ));
    };

    thread.begin_turn(current_workspace)
  };
  let action = turn_actions::prepare_turn_action(
    context,
    &prepared_thread.thread_id,
    &params.message,
    prepared_thread.workspace.as_ref(),
    &permission_sources,
    local_execution_safety_mode,
    cancellation.clone(),
  );
  let reserved_approval_ids =
    reserve_follow_up_approval_ids(context, &permission_sources, local_execution_safety_mode);
  if context
    .execution_state
    .take_pending_running_cancel(&prepared_thread.thread_id)
  {
    cancellation.cancel();
  }
  context.execution_state.insert_running_turn(
    prepared_thread.turn_id.clone(),
    prepared_thread.thread_id.clone(),
    cancellation.clone(),
  );

  Ok(PreparedTurnStart {
    request_id: request.id,
    snapshot: PreparedTurnSnapshot {
      thread_id: prepared_thread.thread_id,
      turn_id: prepared_thread.turn_id,
      thread_title: prepared_thread.thread_title,
      display_message: params.message.clone(),
      message: params.message,
      workspace: prepared_thread.workspace,
      model_runtime,
      cancellation,
      memory_notes,
      plugin_state,
      permission_sources,
      local_execution_safety_mode,
      reserved_approval_ids,
      action,
    },
  })
}

fn ensure_turn_model_ready(context: &RuntimeContext) -> std::result::Result<(), String> {
  context.model_state.ensure_ready_for_turn()
}

fn reserve_follow_up_approval_ids(
  context: &mut RuntimeContext,
  permission_sources: &HashMap<String, Vec<String>>,
  local_execution_safety_mode: LocalExecutionSafetyMode,
) -> Vec<String> {
  if !local_execution_safety_mode.should_reserve_approval_id(permission_sources, "file.write")
    && !local_execution_safety_mode.should_reserve_approval_id(permission_sources, "shell.exec")
    && context.plugin_state.enabled_command_capability_count() == 0
  {
    return vec![];
  }

  (0..LOOP_MAX_STEPS)
    .map(|_| context.sequence_state.next_approval_id())
    .collect()
}

pub fn execute_prepared_turn_start(prepared: PreparedTurnStart) -> CompletedTurnStart {
  let request_id = prepared.request_id;
  let snapshot = prepared.snapshot;
  let thread_id = snapshot.thread_id.clone();
  let turn_id = snapshot.turn_id.clone();
  let display_message = snapshot.display_message.clone();
  let output = catch_unwind(AssertUnwindSafe(|| {
    turn_actions::execute_prepared_turn_snapshot(snapshot)
  }))
  .unwrap_or_else(|_| {
    turn_actions::build_recovered_turn_output(thread_id, turn_id, display_message)
  });

  CompletedTurnStart { request_id, output }
}

pub fn complete_prepared_turn_start(
  context: &mut RuntimeContext,
  completed: CompletedTurnStart,
) -> JsonRpcResponse {
  let mut output = completed.output;
  let plugin_command_outputs = std::mem::take(&mut output.plugin_command_outputs);
  context.execution_state.remove_running_turn(&output.turn_id);
  let was_cancelled = output.items.iter().any(|item| {
    item
      .attributes
      .as_ref()
      .and_then(|attributes| attributes.get("streamingStatus"))
      .map(|status| status == "cancelled")
      .unwrap_or(false)
  });
  let active_turn_id = output
    .pending_active_turn
    .as_ref()
    .map(|turn| turn.id().to_string());

  if let Some(approval) = output.pending_approval.clone() {
    context.execution_state.insert_pending_approval(approval);
  }

  let Some(thread) = context.thread_state.find_mut(&output.thread_id) else {
    return JsonRpcResponse::error(completed.request_id, -32004, "Session not found");
  };

  if was_cancelled {
    thread.mark_cancelled();
  } else if active_turn_id.is_some() {
    thread.mark_streaming();
  } else if !thread.status_contains("approval") {
    thread.mark_ready();
  }
  thread.append_items(output.items.clone());

  if let Some(active_turn) = output.pending_active_turn {
    context.execution_state.insert_active_turn(active_turn);
  }

  if let Err(error) = context.persist_runtime_state() {
    return JsonRpcResponse::error(completed.request_id, -32010, error.to_string());
  }

  let mut plugin_memory_items = vec![];
  for plugin_output in plugin_command_outputs {
    plugin_memory_items.extend(capture_plugin_command_output_memory(
      context,
      &output.thread_id,
      &plugin_output.command,
      plugin_output.workspace.as_ref(),
      plugin_output.input.as_deref(),
      &plugin_output.items,
      plugin_output.capture_memory,
      &plugin_output.runner_memory_notes,
    ));
  }
  if !plugin_memory_items.is_empty() {
    if let Some(thread) = context.thread_state.find_mut(&output.thread_id) {
      thread.append_items(plugin_memory_items.clone());
    }
    output.items.extend(plugin_memory_items);
    if let Err(error) = context.persist_runtime_state() {
      return JsonRpcResponse::error(completed.request_id, -32010, error.to_string());
    }
  }

  if active_turn_id.is_none() {
    if let Err(error) = refresh_thread_summary_note(context, &output.thread_id) {
      return JsonRpcResponse::error(completed.request_id, -32012, error.to_string());
    }
  }

  JsonRpcResponse::success(
    completed.request_id,
    &TurnStartResult {
      turn_id: output.turn_id,
      thread_id: output.thread_id.clone(),
      items: output.items,
      pending_approvals: approvals_for_thread(context, &output.thread_id),
      active_turn_id,
    },
  )
}
