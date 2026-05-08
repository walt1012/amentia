use anyhow::Result;
use pith_protocol::{methods, JsonRpcNotification};

use crate::runtime_context::RuntimeContext;

use super::turn_streaming_progress::advance_active_turn;

pub(crate) use super::turn_cancel::{
  build_turn_cancelled_items, handle_turn_cancel, handle_turn_cancel_running,
};

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
