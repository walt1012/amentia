use pith_protocol::{
  JsonRpcRequest, JsonRpcResponse, ThreadListResult, ThreadReadParams, ThreadReadResult,
  ThreadStartParams, ThreadStartResult, ThreadSummary, TimelineItem,
};

use crate::active_turns::active_turn_id_for_thread;
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
      threads: context
        .threads
        .iter()
        .map(|thread| thread.summary.clone())
        .collect(),
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

  let Some(thread) = context
    .threads
    .iter()
    .find(|thread| thread.summary.id == params.thread_id)
  else {
    return JsonRpcResponse::error(request.id, -32004, "Thread not found");
  };

  JsonRpcResponse::success(
    request.id,
    &ThreadReadResult {
      thread: thread.summary.clone(),
      items: thread.items.clone(),
      pending_approvals: approvals_for_thread(context, &thread.summary.id),
      active_turn_id: active_turn_id_for_thread(&context.active_turns, &thread.summary.id),
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

  let workspace = context.workspace.clone();
  let thread = ThreadSummary {
    id: context.sequences.next_thread_id(),
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
  context.threads.push(StoredThread {
    summary: thread.clone(),
    turn_count: 0,
    items,
    workspace,
  });

  if let Err(error) = context.persist_runtime_state() {
    return JsonRpcResponse::error(request.id, -32010, error.to_string());
  }

  JsonRpcResponse::success(request.id, &ThreadStartResult { thread })
}
