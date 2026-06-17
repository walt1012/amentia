use anyhow::Result;
use amentia_protocol::ThreadUpdatedNotificationParams;

use crate::active_turns::{streaming_progress_label, update_streaming_item};
use crate::approval_state::approvals_for_thread;
use crate::runtime_context::RuntimeContext;
use crate::text_utils::take_characters;
use crate::thread_summary::refresh_thread_summary_note;

pub(super) fn advance_active_turn(
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
