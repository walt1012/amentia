use pith_protocol::{JsonRpcRequest, JsonRpcResponse, TurnStartParams, TurnStartResult};

use crate::approval_state::approvals_for_thread;
use crate::plugin_permissions::granted_permission_sources;
use crate::request_params::parse_required_params;
use crate::request_state::{CompletedTurnStart, PreparedTurnSnapshot, PreparedTurnStart};
use crate::runtime_context::RuntimeContext;
use crate::thread_summary::refresh_thread_summary_note;
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

  let current_workspace = context.workspace_state.current.clone();
  let model_runtime = context.model_runtime.clone();
  let memory_notes = context.memory_state.notes().to_vec();
  let permission_sources = granted_permission_sources(&context.plugin_state.catalog);
  let (thread_id, turn_id, thread_title, workspace) = {
    let Some(thread) = context
      .threads
      .iter_mut()
      .find(|thread| thread.summary.id == params.thread_id)
    else {
      return Err(JsonRpcResponse::error(
        request.id,
        -32004,
        "Thread not found",
      ));
    };

    thread.turn_count += 1;
    let turn_count = thread.turn_count;
    if thread.workspace.is_none() {
      thread.workspace = current_workspace.clone();
      thread.summary.workspace = thread.workspace.clone();
    }
    let workspace = thread.workspace.clone();
    thread.summary.status = match &workspace {
      Some(workspace) => format!("{turn_count} turn(s) in {}", workspace.display_name),
      None => format!("{turn_count} turn(s)"),
    };

    (
      thread.summary.id.clone(),
      format!("{}-turn-{turn_count}", thread.summary.id),
      thread.summary.title.clone(),
      workspace,
    )
  };
  let action = turn_actions::prepare_turn_action(
    context,
    &params.message,
    workspace.as_ref(),
    &permission_sources,
  );

  Ok(PreparedTurnStart {
    request_id: request.id,
    snapshot: PreparedTurnSnapshot {
      thread_id,
      turn_id,
      thread_title,
      display_message: params.message.clone(),
      message: params.message,
      workspace,
      model_runtime,
      memory_notes,
      permission_sources,
      action,
    },
  })
}

fn ensure_turn_model_ready(context: &RuntimeContext) -> std::result::Result<(), String> {
  if !context.enforce_model_readiness {
    return Ok(());
  }

  let health = context.model_runtime.health();
  if health.status == "ready" {
    return Ok(());
  }

  Err(format!(
    "Local model is not ready for turn/start. Download and activate a local model first. {}",
    health.detail
  ))
}

pub fn execute_prepared_turn_start(prepared: PreparedTurnStart) -> CompletedTurnStart {
  CompletedTurnStart {
    request_id: prepared.request_id,
    output: turn_actions::execute_prepared_turn_snapshot(prepared.snapshot),
  }
}

pub fn complete_prepared_turn_start(
  context: &mut RuntimeContext,
  completed: CompletedTurnStart,
) -> JsonRpcResponse {
  let output = completed.output;
  let active_turn_id = output
    .pending_active_turn
    .as_ref()
    .map(|turn| turn.id.clone());

  if let Some(approval) = output.pending_approval.clone() {
    context.execution_state.insert_pending_approval(approval);
  }

  let Some(thread) = context
    .threads
    .iter_mut()
    .find(|thread| thread.summary.id == output.thread_id)
  else {
    return JsonRpcResponse::error(completed.request_id, -32004, "Thread not found");
  };

  if active_turn_id.is_some() {
    thread.summary.status = "Streaming assistant response".to_string();
  } else if !thread.summary.status.contains("approval") {
    thread.summary.status = "Ready".to_string();
  }
  thread.items.extend(output.items.clone());

  if let Some(active_turn) = output.pending_active_turn {
    context.execution_state.insert_active_turn(active_turn);
  }

  if let Err(error) = context.persist_runtime_state() {
    return JsonRpcResponse::error(completed.request_id, -32010, error.to_string());
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
