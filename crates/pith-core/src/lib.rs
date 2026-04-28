use approval_state::approvals_for_thread;
use pith_protocol::{methods, JsonRpcRequest, JsonRpcResponse, TurnStartParams, TurnStartResult};
use plugin_permissions::granted_permission_sources;
use request_params::parse_required_params;
use runtime_context::PreparedTurnSnapshot;
pub use runtime_context::{
  CompletedApprovalRespond, CompletedTurnStart, PreparedApprovalRespond, PreparedTurnStart,
  RuntimeContext,
};
use runtime_readiness::build_runtime_readiness;
use thread_summary::refresh_thread_summary_note;

mod active_turns;
mod approval_requests;
mod approval_state;
mod context_compaction;
mod context_state;
mod intent_inference;
mod local_responses;
mod memory_requests;
mod model_requests;
mod plugin_catalog_state;
mod plugin_commands;
mod plugin_hooks;
mod plugin_permissions;
mod plugin_requests;
mod protocol_adapters;
mod request_params;
mod runtime_context;
mod runtime_readiness;
mod server_requests;
mod text_utils;
mod thread_requests;
mod thread_summary;
mod turn_actions;
mod turn_streaming;
mod workspace_requests;
mod workspace_search;

pub use approval_requests::{
  complete_prepared_approval_respond, execute_prepared_approval_respond, prepare_approval_respond,
};
pub use plugin_commands::{CompletedPluginCommandRun, PreparedPluginCommandRun};
pub use turn_streaming::collect_notifications;
pub use workspace_search::{CompletedWorkspaceSearch, PreparedWorkspaceSearch};

pub fn handle_request(context: &mut RuntimeContext, request: JsonRpcRequest) -> JsonRpcResponse {
  match request.method.as_str() {
    methods::APPROVAL_RESPOND => approval_requests::handle_approval_respond(context, request),
    methods::INITIALIZE => server_requests::handle_initialize(context, request),
    methods::HEALTH_PING => server_requests::handle_health_ping(request),
    methods::MEMORY_CREATE => memory_requests::handle_memory_create(context, request),
    methods::MEMORY_LIST => memory_requests::handle_memory_list(context, request),
    methods::MEMORY_STATUS => memory_requests::handle_memory_status(context, request),
    methods::MODEL_BOOTSTRAP => model_requests::handle_model_bootstrap(context, request),
    methods::MODEL_HEALTH => model_requests::handle_model_health(context, request),
    methods::PLUGIN_CAPABILITY_REGISTRY => {
      plugin_requests::handle_plugin_capability_registry(context, request)
    }
    methods::PLUGIN_COMMAND_REGISTRY => {
      plugin_requests::handle_plugin_command_registry(context, request)
    }
    methods::PLUGIN_COMMAND_RUN => plugin_commands::handle_plugin_command_run(context, request),
    methods::PLUGIN_CONNECTOR_REGISTRY => {
      plugin_requests::handle_plugin_connector_registry(context, request)
    }
    methods::PLUGIN_HOOK_REGISTRY => plugin_requests::handle_plugin_hook_registry(context, request),
    methods::PLUGIN_INSTALL => plugin_requests::handle_plugin_install(context, request),
    methods::PLUGIN_LIST => plugin_requests::handle_plugin_list(context, request),
    methods::PLUGIN_REMOVE => plugin_requests::handle_plugin_remove(context, request),
    methods::PLUGIN_SET_ENABLED => plugin_requests::handle_plugin_set_enabled(context, request),
    methods::RUNTIME_READINESS => {
      JsonRpcResponse::success(request.id, &build_runtime_readiness(context))
    }
    methods::WORKSPACE_CURRENT => workspace_requests::handle_workspace_current(context, request),
    methods::WORKSPACE_OPEN => workspace_requests::handle_workspace_open(context, request),
    methods::WORKSPACE_SEARCH => workspace_search::handle_workspace_search(context, request),
    methods::TURN_CANCEL => turn_streaming::handle_turn_cancel(context, request),
    methods::THREAD_READ => thread_requests::handle_thread_read(context, request),
    methods::THREAD_START => thread_requests::handle_thread_start(context, request),
    methods::THREAD_LIST => thread_requests::handle_thread_list(context, request),
    methods::TURN_START => handle_turn_start(context, request),
    _ => JsonRpcResponse::error(request.id, -32601, "Method not found"),
  }
}

fn handle_turn_start(context: &mut RuntimeContext, request: JsonRpcRequest) -> JsonRpcResponse {
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

  let current_workspace = context.workspace.clone();
  let model_runtime = context.model_runtime.clone();
  let memory_notes = context.memory_notes.clone();
  let permission_sources = granted_permission_sources(&context.plugins);
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
    context
      .pending_approvals
      .insert(approval.id.clone(), approval);
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
    context
      .active_turns
      .insert(active_turn.id.clone(), active_turn);
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

pub fn prepare_workspace_search(
  context: &mut RuntimeContext,
  request: JsonRpcRequest,
) -> std::result::Result<PreparedWorkspaceSearch, JsonRpcResponse> {
  workspace_search::prepare_workspace_search(context, request)
}

pub fn execute_prepared_workspace_search(
  prepared: PreparedWorkspaceSearch,
) -> CompletedWorkspaceSearch {
  workspace_search::execute_prepared_workspace_search(prepared)
}

pub fn complete_prepared_workspace_search(completed: CompletedWorkspaceSearch) -> JsonRpcResponse {
  workspace_search::complete_prepared_workspace_search(completed)
}

pub fn prepare_plugin_command_run(
  context: &mut RuntimeContext,
  request: JsonRpcRequest,
) -> std::result::Result<PreparedPluginCommandRun, JsonRpcResponse> {
  plugin_commands::prepare_plugin_command_run(context, request)
}

pub fn execute_prepared_plugin_command_run(
  prepared: PreparedPluginCommandRun,
) -> CompletedPluginCommandRun {
  plugin_commands::execute_prepared_plugin_command_run(prepared)
}

pub fn complete_prepared_plugin_command_run(
  context: &mut RuntimeContext,
  completed: CompletedPluginCommandRun,
) -> JsonRpcResponse {
  plugin_commands::complete_prepared_plugin_command_run(context, completed)
}

#[cfg(test)]
mod tests;
