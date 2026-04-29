use std::collections::HashMap;

use anyhow::Result;
use pith_protocol::{
  methods, JsonRpcNotification, JsonRpcRequest, JsonRpcResponse, ThreadUpdatedNotificationParams,
  TimelineItem, TurnCancelParams, TurnCancelResult,
};

use crate::active_turns::{streaming_progress_label, update_streaming_item};
use crate::approval_state::approvals_for_thread;
use crate::request_params::parse_required_params;
use crate::runtime_context::RuntimeContext;
use crate::text_utils::take_characters;
use crate::thread_summary::refresh_thread_summary_note;

pub fn collect_notifications(context: &mut RuntimeContext) -> Result<Vec<JsonRpcNotification>> {
  let active_turn_ids = context.execution_state.active_turn_ids();
  let mut notifications = vec![];
  let mut did_update = false;

  for turn_id in active_turn_ids {
    if let Some(params) = advance_active_turn(context, &turn_id)? {
      did_update = true;
      notifications.push(JsonRpcNotification {
        method: methods::THREAD_UPDATED_NOTIFICATION.to_string(),
        params: Some(serde_json::to_value(params)?),
      });
    }
  }

  if did_update {
    context.persist_runtime_state()?;
  }

  Ok(notifications)
}

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

  let items = vec![
    TimelineItem {
      kind: "warning".to_string(),
      title: "Turn Cancelled".to_string(),
      content: format!(
        "Cancelled {} before the assistant response completed.",
        params.turn_id
      ),
      attributes: Some(HashMap::from([(
        "turnId".to_string(),
        params.turn_id.clone(),
      )])),
    },
    TimelineItem {
      kind: "assistantMessage".to_string(),
      title: "Assistant".to_string(),
      content: "Pith stopped the active response at your request.".to_string(),
      attributes: Some(HashMap::from([
        ("turnId".to_string(), params.turn_id.clone()),
        ("streamingStatus".to_string(), "cancelled".to_string()),
      ])),
    },
  ];
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
      turn_id: params.turn_id,
      thread_id: active_turn_snapshot.thread_id().to_string(),
      items,
      active_turn_id: context
        .execution_state
        .active_turn_id_for_thread(&cancelled_thread_id),
    },
  )
}

pub(crate) fn refresh_active_turn_for_thread(
  context: &mut RuntimeContext,
  thread_id: &str,
) -> Result<bool> {
  let active_turn_ids = context
    .execution_state
    .active_turn_ids_for_thread(thread_id);
  let mut did_update = false;

  for turn_id in active_turn_ids {
    if advance_active_turn(context, &turn_id)?.is_some() {
      did_update = true;
    }
  }

  Ok(did_update)
}

fn advance_active_turn(
  context: &mut RuntimeContext,
  turn_id: &str,
) -> Result<Option<ThreadUpdatedNotificationParams>> {
  let Some(snapshot) = context.execution_state.active_turn_snapshot(turn_id) else {
    return Ok(None);
  };
  let target_chars = snapshot.streamed_char_count();

  if target_chars <= snapshot.emitted_chars() {
    return Ok(None);
  }

  let thread_id = snapshot.thread_id().to_string();
  let streamed_content = take_characters(snapshot.full_content(), target_chars);
  let is_complete = target_chars >= snapshot.total_chars();
  let streaming_status = if is_complete {
    "completed"
  } else {
    "in_progress"
  };

  let thread_snapshot = {
    let Some(thread) = context.thread_state.find_mut(snapshot.thread_id()) else {
      return Ok(None);
    };

    update_streaming_item(
      thread.items_mut(),
      turn_id,
      &streamed_content,
      streaming_status,
      target_chars,
      snapshot.total_chars(),
    );

    if is_complete {
      thread.mark_ready();
    } else {
      thread.mark_streaming_progress(streaming_progress_label(
        target_chars,
        snapshot.total_chars(),
      ));
    }

    thread.snapshot()
  };

  if is_complete {
    context.execution_state.remove_active_turn(turn_id);
    refresh_thread_summary_note(context, &thread_id)?;
  } else {
    context
      .execution_state
      .update_active_turn_emitted(turn_id, target_chars);
  }

  Ok(Some(ThreadUpdatedNotificationParams {
    thread: thread_snapshot.0,
    items: thread_snapshot.1,
    pending_approvals: approvals_for_thread(context, &thread_id),
    active_turn_id: context
      .execution_state
      .active_turn_id_for_thread(&thread_id),
  }))
}
