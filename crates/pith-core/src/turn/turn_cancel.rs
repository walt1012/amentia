use std::collections::HashMap;

use pith_protocol::{
  JsonRpcRequest, JsonRpcResponse, TimelineItem, TurnCancelParams, TurnCancelResult,
  TurnCancelRunningParams,
};

use crate::active_turns::update_streaming_item;
use crate::request_params::parse_required_params;
use crate::runtime_context::RuntimeContext;
use crate::text_utils::take_characters;
use crate::thread_summary::refresh_thread_summary_note;

pub(crate) fn handle_turn_cancel(
  context: &mut RuntimeContext,
  request: JsonRpcRequest,
) -> JsonRpcResponse {
  let params = match parse_required_params::<TurnCancelParams>(&request, "turn/cancel") {
    Ok(params) => params,
    Err(response) => return response,
  };

  let Some(active_turn_snapshot) = context
    .execution_state
    .active_turn_snapshot(&params.turn_id)
  else {
    return JsonRpcResponse::error(request.id, -32040, "Turn is not active");
  };

  let Some(thread) = context
    .thread_state
    .find_mut(active_turn_snapshot.thread_id())
  else {
    return JsonRpcResponse::error(request.id, -32004, "Thread not found");
  };
  let cancelled_thread_id = thread.id().to_string();

  context.execution_state.remove_active_turn(&params.turn_id);
  let partial_content = take_characters(
    active_turn_snapshot.full_content(),
    active_turn_snapshot.streamed_char_count(),
  );
  update_streaming_item(
    thread.items_mut(),
    &params.turn_id,
    &partial_content,
    "cancelled",
    partial_content.chars().count(),
    active_turn_snapshot.total_chars(),
  );
  thread.mark_cancelled();

  let items = build_turn_cancelled_items(&params.turn_id);
  thread.append_items(items.clone());

  if let Err(error) = context.persist_threads() {
    return JsonRpcResponse::error(request.id, -32010, error.to_string());
  }

  if let Err(error) = refresh_thread_summary_note(context, &cancelled_thread_id) {
    return JsonRpcResponse::error(request.id, -32012, error.to_string());
  }

  JsonRpcResponse::success(
    request.id,
    &TurnCancelResult {
      turn_id: Some(params.turn_id),
      thread_id: active_turn_snapshot.thread_id().to_string(),
      items,
      active_turn_id: context
        .execution_state
        .active_turn_id_for_thread(&cancelled_thread_id),
    },
  )
}

pub(crate) fn handle_turn_cancel_running(
  context: &mut RuntimeContext,
  request: JsonRpcRequest,
) -> JsonRpcResponse {
  let params =
    match parse_required_params::<TurnCancelRunningParams>(&request, "turn/cancelRunning") {
      Ok(params) => params,
      Err(response) => return response,
    };

  if context.thread_state.find(&params.thread_id).is_none() {
    return JsonRpcResponse::error(request.id, -32004, "Thread not found");
  }

  let cancellation = context
    .execution_state
    .request_running_cancel_for_thread(&params.thread_id);
  let thread_id = cancellation
    .as_ref()
    .map(|cancellation| cancellation.thread_id().to_string())
    .unwrap_or_else(|| params.thread_id.clone());
  let turn_id = cancellation
    .as_ref()
    .and_then(|cancellation| cancellation.turn_id())
    .map(str::to_string);

  JsonRpcResponse::success(
    request.id,
    &TurnCancelResult {
      turn_id,
      thread_id: thread_id.clone(),
      items: vec![],
      active_turn_id: context
        .execution_state
        .active_turn_id_for_thread(&thread_id),
    },
  )
}

pub(crate) fn build_turn_cancelled_items(turn_id: &str) -> Vec<TimelineItem> {
  vec![
    TimelineItem {
      kind: "warning".to_string(),
      title: "Turn Cancelled".to_string(),
      content: format!("Cancelled {turn_id} before the assistant response completed."),
      attributes: Some(HashMap::from([("turnId".to_string(), turn_id.to_string())])),
    },
    TimelineItem {
      kind: "assistantMessage".to_string(),
      title: "Assistant".to_string(),
      content: "Pith stopped the local response at your request.".to_string(),
      attributes: Some(HashMap::from([
        ("turnId".to_string(), turn_id.to_string()),
        ("streamingStatus".to_string(), "cancelled".to_string()),
      ])),
    },
  ]
}
