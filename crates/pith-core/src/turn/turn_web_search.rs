use std::collections::HashMap;

use pith_protocol::TimelineItem;
use pith_tools::web_search_with_cancellation;

use crate::active_turns::{start_streaming_assistant_turn, ActiveTurn};
use crate::intent_inference::WebSearchIntent;
use crate::local_responses::{
  build_plan_item, format_web_search_result, summarize_web_search_result,
};
use crate::plugin_permissions::{build_permission_denied_items, permission_is_granted};
use crate::request_state::PreparedTurnSnapshot;

const WEB_SEARCH_MAX_RESULTS: usize = 5;

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
        "Check plugin permissions before searching the web for \"{}\".",
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

  items.push(TimelineItem {
    kind: "toolStart".to_string(),
    title: "web_search".to_string(),
    content: query.to_string(),
    attributes: Some(HashMap::from([
      ("tool".to_string(), "web_search".to_string()),
      ("provider".to_string(), "DuckDuckGo Lite".to_string()),
      ("networkAccess".to_string(), "true".to_string()),
      (
        "routingReason".to_string(),
        intent.routing_reason.to_string(),
      ),
    ])),
  });

  match web_search_with_cancellation(query, WEB_SEARCH_MAX_RESULTS, || {
    snapshot.cancellation.is_cancelled()
  }) {
    Ok(results) => {
      items.push(TimelineItem {
        kind: "toolResult".to_string(),
        title: "web_search result".to_string(),
        content: format_web_search_result(query, &results),
        attributes: Some(HashMap::from([
          ("tool".to_string(), "web_search".to_string()),
          ("provider".to_string(), "DuckDuckGo Lite".to_string()),
          ("resultCount".to_string(), results.len().to_string()),
          (
            "routingReason".to_string(),
            intent.routing_reason.to_string(),
          ),
        ])),
      });
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
      items.push(TimelineItem {
        kind: "warning".to_string(),
        title: "web_search failed".to_string(),
        content: error.to_string(),
        attributes: Some(HashMap::from([
          ("tool".to_string(), "web_search".to_string()),
          ("provider".to_string(), "DuckDuckGo Lite".to_string()),
          (
            "routingReason".to_string(),
            intent.routing_reason.to_string(),
          ),
        ])),
      });
      items.push(TimelineItem {
        kind: "assistantMessage".to_string(),
        title: "Assistant".to_string(),
        content: "Pith could not search the web yet. Check network access and try again."
          .to_string(),
        attributes: None,
      });
    }
  }
}
