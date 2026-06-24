use std::collections::{BTreeSet, HashMap};
use std::path::Path;

use amentia_protocol::{TimelineItem, WorkspaceSummary};
use amentia_tools::search_files_with_cancellation;

use super::turn_tool_limits::SEARCH_FILES_RESULT_LIMIT;
use super::turn_workspace_timeline::{
  workspace_tool_failed_items, workspace_tool_result_item, workspace_tool_start_item,
};
use crate::active_turn_model::ActiveTurn;
use crate::active_turn_timeline::start_streaming_assistant_turn;
use crate::local_responses::{build_plan_item, format_search_result, summarize_search_result};
use crate::plugin_permission_denied::build_permission_denied_items;
use crate::plugin_permission_sources::permission_is_granted;
use crate::request_state::PreparedTurnSnapshot;

pub(super) fn execute_search_observation_step(
  snapshot: &PreparedTurnSnapshot,
  workspace: &WorkspaceSummary,
  query: &str,
  items: &mut Vec<TimelineItem>,
  pending_active_turn: &mut Option<ActiveTurn>,
) -> Option<String> {
  execute_search_step(snapshot, workspace, query, items, pending_active_turn)
}

fn execute_search_step(
  snapshot: &PreparedTurnSnapshot,
  workspace: &WorkspaceSummary,
  query: &str,
  items: &mut Vec<TimelineItem>,
  pending_active_turn: &mut Option<ActiveTurn>,
) -> Option<String> {
  items.push(build_plan_item(
    &snapshot.model_runtime,
    &snapshot.memory_notes,
    &snapshot.plugin_skill_context,
    &snapshot.message,
    Some(workspace),
    if permission_is_granted(&snapshot.permission_sources, "file.read") {
      format!(
        "Search {} for matches to \"{}\" with the built-in search_files tool.",
        workspace.display_name, query
      )
    } else {
      format!(
        "Check plugin permissions before searching {} for \"{}\".",
        workspace.display_name, query
      )
    },
    Some(&snapshot.cancellation),
  ));
  if snapshot.cancellation.is_cancelled() {
    items.extend(crate::turn_streaming::build_turn_cancelled_items(
      &snapshot.turn_id,
    ));
    return None;
  }
  if !permission_is_granted(&snapshot.permission_sources, "file.read") {
    items.extend(build_permission_denied_items(
      &snapshot.permission_sources,
      "file.read",
      "search files",
      &workspace.display_name,
      HashMap::from([("query".to_string(), query.to_string())]),
    ));
    return None;
  }

  items.push(workspace_tool_start_item(
    "search_files",
    query.to_string(),
    workspace,
    [
      ("query".to_string(), query.to_string()),
      (
        "maxResults".to_string(),
        SEARCH_FILES_RESULT_LIMIT.to_string(),
      ),
    ],
  ));

  match search_files_with_cancellation(
    Path::new(&workspace.root_path),
    query,
    SEARCH_FILES_RESULT_LIMIT,
    || snapshot.cancellation.is_cancelled(),
  ) {
    Ok(matches) => {
      let next_read_path = single_result_path(&matches);
      items.push(workspace_tool_result_item(
        "search_files",
        format_search_result(query, &matches),
        workspace,
        search_result_attributes(
          query,
          matches.len(),
          unique_result_path_count(&matches),
          next_read_path.as_deref(),
        ),
      ));
      if next_read_path.is_some() {
        return next_read_path;
      }
      let (summary, summary_attributes) = summarize_search_result(
        &snapshot.model_runtime,
        &snapshot.memory_notes,
        &snapshot.thread_title,
        &workspace.display_name,
        query,
        &matches,
        Some(&snapshot.cancellation),
      );
      if snapshot.cancellation.is_cancelled() {
        items.extend(crate::turn_streaming::build_turn_cancelled_items(
          &snapshot.turn_id,
        ));
        return None;
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
        return None;
      }
      items.extend(workspace_tool_failed_items(
        "search_files",
        error.to_string(),
        format!(
          "Amentia could not search {} yet. Try a shorter query or re-open the workspace.",
          workspace.display_name
        ),
        workspace,
        [("query".to_string(), query.to_string())],
      ));
    }
  }

  None
}

fn search_result_attributes(
  query: &str,
  result_count: usize,
  unique_path_count: usize,
  next_read_path: Option<&str>,
) -> Vec<(String, String)> {
  let mut attributes = vec![
    ("query".to_string(), query.to_string()),
    ("resultCount".to_string(), result_count.to_string()),
    ("uniquePathCount".to_string(), unique_path_count.to_string()),
    (
      "maxResults".to_string(),
      SEARCH_FILES_RESULT_LIMIT.to_string(),
    ),
  ];
  if let Some(path) = next_read_path {
    attributes.push(("nextAction".to_string(), "read_file".to_string()));
    attributes.push(("nextRelativePath".to_string(), path.to_string()));
  }

  attributes
}

fn single_result_path(matches: &[amentia_tools::SearchMatch]) -> Option<String> {
  let first_path = matches.first()?.relative_path.as_str();
  matches
    .iter()
    .all(|entry| entry.relative_path == first_path)
    .then(|| first_path.to_string())
}

fn unique_result_path_count(matches: &[amentia_tools::SearchMatch]) -> usize {
  matches
    .iter()
    .map(|entry| entry.relative_path.as_str())
    .collect::<BTreeSet<_>>()
    .len()
}
