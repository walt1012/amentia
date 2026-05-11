use std::collections::HashMap;

use pith_protocol::TimelineItem;
use pith_tools::{web_search_status, web_search_with_cancellation};

use super::turn_tool_limits::WEB_SEARCH_RESULT_LIMIT;
use super::turn_web_search_timeline::{
  web_search_failed_items, web_search_result_item, web_search_start_item,
  web_search_unavailable_items,
};
use crate::active_turns::{start_streaming_assistant_turn, ActiveTurn};
use crate::intent_inference::WebSearchIntent;
use crate::local_responses::{
  build_plan_item, format_web_search_result, summarize_web_search_result,
};
use crate::plugin_permissions::{build_permission_denied_items, permission_is_granted};
use crate::request_state::PreparedTurnSnapshot;

pub(super) fn execute_web_search_turn(
  snapshot: &PreparedTurnSnapshot,
  intent: &WebSearchIntent,
  items: &mut Vec<TimelineItem>,
  pending_active_turn: &mut Option<ActiveTurn>,
) {
  let query = intent.query.as_str();
  items.push(build_plan_item(
    &snapshot.model_runtime,
    &snapshot.memory_notes,
    &snapshot.message,
    snapshot.workspace.as_ref(),
    if permission_is_granted(&snapshot.permission_sources, "network.outbound") {
      format!(
        "Search the web for \"{}\" with the built-in web_search tool.",
        query
      )
    } else {
      format!(
        "Check network permission before searching the web for \"{}\".",
        query
      )
    },
    Some(&snapshot.cancellation),
  ));
  if snapshot.cancellation.is_cancelled() {
    items.extend(crate::turn_streaming::build_turn_cancelled_items(
      &snapshot.turn_id,
    ));
    return;
  }
  if !permission_is_granted(&snapshot.permission_sources, "network.outbound") {
    items.extend(build_permission_denied_items(
      &snapshot.permission_sources,
      "network.outbound",
      "search the web",
      "the web",
      HashMap::from([
        ("query".to_string(), query.to_string()),
        (
          "routingReason".to_string(),
          intent.routing_reason.to_string(),
        ),
      ]),
    ));
    return;
  }

  let search_status = web_search_status();
  if !search_status.available {
    items.extend(web_search_unavailable_items(intent, &search_status));
    return;
  }

  items.push(web_search_start_item(intent, &search_status));

  match web_search_with_cancellation(query, WEB_SEARCH_RESULT_LIMIT, || {
    snapshot.cancellation.is_cancelled()
  }) {
    Ok(results) => {
      items.push(web_search_result_item(
        intent,
        &search_status,
        format_web_search_result(query, &results),
        results.len(),
      ));
      let (summary, summary_attributes) = summarize_web_search_result(
        &snapshot.model_runtime,
        &snapshot.memory_notes,
        &snapshot.thread_title,
        query,
        &results,
        Some(&snapshot.cancellation),
      );
      if snapshot.cancellation.is_cancelled() {
        items.extend(crate::turn_streaming::build_turn_cancelled_items(
          &snapshot.turn_id,
        ));
        return;
      }
      *pending_active_turn = start_streaming_assistant_turn(
        &snapshot.thread_id,
        &snapshot.turn_id,
        items,
        summary,
        summary_attributes,
      );
    }
    Err(error) => {
      if snapshot.cancellation.is_cancelled() {
        items.extend(crate::turn_streaming::build_turn_cancelled_items(
          &snapshot.turn_id,
        ));
        return;
      }
      items.extend(web_search_failed_items(
        intent,
        &search_status,
        error.to_string(),
      ));
    }
  }
}
