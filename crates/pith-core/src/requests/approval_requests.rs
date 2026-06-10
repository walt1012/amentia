use pith_model_runtime::GenerationCancellation;
use pith_protocol::{ApprovalRespondParams, JsonRpcRequest, JsonRpcResponse};

pub use super::approval_completion::complete_prepared_approval_respond;
pub use super::approval_execution::execute_prepared_approval_respond;

use crate::plugin_commands::prepare_approved_plugin_command_snapshot;
use crate::plugin_permissions::granted_permission_sources;
use crate::request_params::parse_required_params;
use crate::request_state::{PreparedApprovalRespond, PreparedApprovalSnapshot};
use crate::requests::approval_agent_context::ApprovalAgentContext;
use crate::runtime_context::RuntimeContext;

pub(crate) fn handle_approval_respond(
  context: &mut RuntimeContext,
  request: JsonRpcRequest,
) -> JsonRpcResponse {
  let prepared = match prepare_approval_respond(context, request) {
    Ok(prepared) => prepared,
    Err(response) => return response,
  };
  let completed = execute_prepared_approval_respond(prepared);
  complete_prepared_approval_respond(context, completed)
}

pub fn prepare_approval_respond(
  context: &mut RuntimeContext,
  request: JsonRpcRequest,
) -> std::result::Result<PreparedApprovalRespond, JsonRpcResponse> {
  let params = parse_required_params::<ApprovalRespondParams>(&request, "approval/respond")?;
  let decision = params.decision.to_lowercase();
  if decision != "approved" && decision != "denied" {
    return Err(JsonRpcResponse::error(
      request.id,
      -32602,
      "approval/respond decision must be approved or denied",
    ));
  }

  let Some(approval) = context
    .execution_state
    .pending_approval_snapshot(&params.approval_id)
  else {
    return Err(JsonRpcResponse::error(
      request.id,
      -32030,
      "Approval request not found",
    ));
  };
  let current_workspace = context.workspace_state.current_cloned();
  let model_runtime = context.model_state.snapshot();
  let cancellation = GenerationCancellation::new();
  let memory_notes = context.memory_state.snapshot_notes();
  let permission_sources = granted_permission_sources(context.plugin_state.catalog());
  let plugins = context.plugin_state.snapshot_catalog();

  let (workspace, agent_context) = {
    let Some(thread) = context.thread_state.find_mut(&approval.thread_id) else {
      return Err(JsonRpcResponse::error(
        request.id,
        -32004,
        "Thread not found",
      ));
    };
    thread.bind_workspace_if_missing(current_workspace);
    let Some(workspace) = thread.workspace_cloned() else {
      return Err(JsonRpcResponse::error(
        request.id,
        -32031,
        "Open a workspace for this thread before resolving approvals",
      ));
    };
    let agent_context = ApprovalAgentContext::from_thread_items(&approval, thread.items());
    (workspace, agent_context)
  };
  if context
    .execution_state
    .take_pending_running_cancel(&approval.thread_id)
  {
    cancellation.cancel();
  }
  let approved_plugin_command = if decision == "approved" {
    match prepare_approved_plugin_command_snapshot(
      context,
      &approval,
      Some(workspace.clone()),
      cancellation.clone(),
    ) {
      Ok(snapshot) => snapshot,
      Err(error) => return Err(error.into_response(request.id)),
    }
  } else {
    None
  };
  if let Some(thread) = context.thread_state.find_mut(&approval.thread_id) {
    thread.mark_resolving_approval(&approval.id);
  } else {
    return Err(JsonRpcResponse::error(
      request.id,
      -32004,
      "Thread not found",
    ));
  }
  context
    .execution_state
    .remove_pending_approval(&params.approval_id);
  context.execution_state.insert_running_approval(
    approval.id.clone(),
    approval.thread_id.clone(),
    cancellation.clone(),
  );

  Ok(PreparedApprovalRespond {
    request_id: request.id,
    snapshot: PreparedApprovalSnapshot {
      approval,
      decision,
      workspace,
      agent_context,
      model_runtime,
      cancellation,
      memory_notes,
      permission_sources,
      plugins,
      approved_plugin_command,
    },
  })
}
