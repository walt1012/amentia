use std::collections::HashMap;

use pith_model_runtime::{GenerateRequest, ModelRole};
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
  build_plan_item, format_web_search_result, summarize_declined_web_search_candidate,
  summarize_web_search_result,
};
use crate::plugin_permissions::{
  build_permission_denied_items, permission_is_granted, WEB_SEARCH_TOOL_PERMISSION,
};
use crate::request_state::PreparedTurnSnapshot;

const WEB_SEARCH_ROUTE_DECISION_TOKENS: usize = 8;

pub(super) fn model_confirms_web_search_candidate(
  snapshot: &PreparedTurnSnapshot,
  intent: &WebSearchIntent,
) -> bool {
  if snapshot.cancellation.is_cancelled() {
    return true;
  }

  let response = snapshot.model_runtime.generate(GenerateRequest {
    role: ModelRole::Planner,
    prompt: format!(
      "Choose whether Pith should use web_search before answering.\n\
       Return exactly WEB_SEARCH or NO_SEARCH.\n\
       Use WEB_SEARCH for current public facts, external docs, prices, news, releases, weather, schedules, or explicit online lookup.\n\
       Use NO_SEARCH for local workspace, files, repo state, connector state, or reasoning tasks.\n\
       User request: {}\n\
       Candidate reason: {}\n\
       Decision:",
      snapshot.message,
      intent.routing_reason
    ),
    max_tokens: WEB_SEARCH_ROUTE_DECISION_TOKENS,
    cancellation: Some(snapshot.cancellation.clone()),
  });

  if response.status != "ready" {
    return intent.routing_reason != "modelToolPlanning";
  }

  let decision = response.text.trim().to_ascii_uppercase();
  if decision.contains("NO_SEARCH") {
    return false;
  }
  decision.contains("WEB_SEARCH") || decision.contains("YES")
}

pub(super) fn execute_web_search_candidate_local_answer(
  snapshot: &PreparedTurnSnapshot,
  intent: &WebSearchIntent,
  items: &mut Vec<TimelineItem>,
  pending_active_turn: &mut Option<ActiveTurn>,
) {
  items.push(build_plan_item(
    &snapshot.model_runtime,
    &snapshot.memory_notes,
    &snapshot.message,
    snapshot.workspace.as_ref(),
    format!(
      "Answer directly because the local planner declined web_search for \"{}\".",
      intent.query
    ),
    Some(&snapshot.cancellation),
  ));
  if snapshot.cancellation.is_cancelled() {
    items.extend(crate::turn_streaming::build_turn_cancelled_items(
      &snapshot.turn_id,
    ));
    return;
  }

  let (summary, mut summary_attributes) = summarize_declined_web_search_candidate(
    &snapshot.model_runtime,
    &snapshot.memory_notes,
    &snapshot.thread_title,
    &snapshot.message,
    intent,
    Some(&snapshot.cancellation),
  );
  if snapshot.cancellation.is_cancelled() {
    items.extend(crate::turn_streaming::build_turn_cancelled_items(
      &snapshot.turn_id,
    ));
    return;
  }
  summary_attributes.insert("webSearchDecision".to_string(), "declined".to_string());
  summary_attributes.insert(
    "routingReason".to_string(),
    intent.routing_reason.to_string(),
  );
  summary_attributes.insert("query".to_string(), intent.query.to_string());
  *pending_active_turn = start_streaming_assistant_turn(
    &snapshot.thread_id,
    &snapshot.turn_id,
    items,
    summary,
    summary_attributes,
  );
}

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
    if permission_is_granted(&snapshot.permission_sources, WEB_SEARCH_TOOL_PERMISSION) {
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
  if !permission_is_granted(&snapshot.permission_sources, WEB_SEARCH_TOOL_PERMISSION) {
    items.extend(build_permission_denied_items(
      &snapshot.permission_sources,
      WEB_SEARCH_TOOL_PERMISSION,
      "search the web",
      "the web",
      HashMap::from([
        ("pluginId".to_string(), "web-search".to_string()),
        ("pluginDisplayName".to_string(), "Web Search".to_string()),
        (
          "permissionGate".to_string(),
          "requiresPluginPermission".to_string(),
        ),
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
