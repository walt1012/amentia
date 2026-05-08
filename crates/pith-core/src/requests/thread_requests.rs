use pith_protocol::{
  JsonRpcRequest, JsonRpcResponse, ThreadListResult, ThreadReadParams, ThreadReadResult,
  ThreadStartParams, ThreadStartResult, ThreadSummary, TimelineItem,
};

use crate::approval_state::approvals_for_thread;
use crate::request_params::parse_required_params;
use crate::thread_state::StoredThread;
use crate::turn_streaming::refresh_active_turn_for_thread;
use crate::RuntimeContext;

pub(crate) fn handle_thread_list(
  context: &RuntimeContext,
  request: JsonRpcRequest,
) -> JsonRpcResponse {
  JsonRpcResponse::success(
    request.id,
    &ThreadListResult {
      threads: context.thread_state.summaries(),
    },
  )
}

pub(crate) fn handle_thread_read(
  context: &mut RuntimeContext,
  request: JsonRpcRequest,
) -> JsonRpcResponse {
  let params = match parse_required_params::<ThreadReadParams>(&request, "thread/read") {
    Ok(params) => params,
    Err(response) => return response,
  };

  let did_refresh = match refresh_active_turn_for_thread(context, &params.thread_id) {
    Ok(did_refresh) => did_refresh,
    Err(error) => {
      return JsonRpcResponse::error(request.id, -32010, error.to_string());
    }
  };

  if did_refresh {
    if let Err(error) = context.persist_runtime_state() {
      return JsonRpcResponse::error(request.id, -32010, error.to_string());
    }
  }

  let Some((thread, items)) = context.thread_state.snapshot(&params.thread_id) else {
    return JsonRpcResponse::error(request.id, -32004, "Thread not found");
  };
  let thread_id = thread.id.clone();

  JsonRpcResponse::success(
    request.id,
    &ThreadReadResult {
      pending_approvals: approvals_for_thread(context, &thread_id),
      active_turn_id: context
        .execution_state
        .active_turn_id_for_thread(&thread_id),
      thread,
      items,
    },
  )
}

pub(crate) fn handle_thread_start(
  context: &mut RuntimeContext,
  request: JsonRpcRequest,
) -> JsonRpcResponse {
  let params = match parse_required_params::<ThreadStartParams>(&request, "thread/start") {
    Ok(params) => params,
    Err(response) => return response,
  };

  let workspace = context.workspace_state.current_cloned();
  let thread = ThreadSummary {
    id: context.sequence_state.next_thread_id(),
    title: params.title,
    status: "ready".to_string(),
    workspace: workspace.clone(),
  };
  let items = vec![TimelineItem {
    kind: "system".to_string(),
    title: "Thread Ready".to_string(),
    content: format!("{} is ready for local runtime messages.", thread.title),
    attributes: None,
  }];
  context
    .thread_state
    .push(StoredThread::new(thread.clone(), 0, items, workspace));

  if let Err(error) = context.persist_runtime_state() {
    return JsonRpcResponse::error(request.id, -32010, error.to_string());
  }

  JsonRpcResponse::success(request.id, &ThreadStartResult { thread })
}
