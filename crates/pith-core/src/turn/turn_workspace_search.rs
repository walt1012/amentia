use std::collections::HashMap;
use std::path::Path;

use pith_protocol::{TimelineItem, WorkspaceSummary};
use pith_tools::search_files;

use crate::active_turns::{start_streaming_assistant_turn, ActiveTurn};
use crate::local_responses::{build_plan_item, format_search_result, summarize_search_result};
use crate::plugin_permissions::{build_permission_denied_items, permission_is_granted};
use crate::request_state::PreparedTurnSnapshot;
use super::turn_tool_provenance::workspace_tool_attributes;

pub(super) fn execute_search_turn(
  snapshot: &PreparedTurnSnapshot,
  workspace: &WorkspaceSummary,
  query: &str,
  items: &mut Vec<TimelineItem>,
  pending_active_turn: &mut Option<ActiveTurn>,
) {
  items.push(build_plan_item(
    &snapshot.model_runtime,
    &snapshot.memory_notes,
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
    return;
  }
  if !permission_is_granted(&snapshot.permission_sources, "file.read") {
    items.extend(build_permission_denied_items(
      &snapshot.permission_sources,
      "file.read",
      "search files",
      &workspace.display_name,
      HashMap::from([("query".to_string(), query.to_string())]),
    ));
    return;
  }

  items.push(TimelineItem {
    kind: "toolStart".to_string(),
    title: "search_files".to_string(),
    content: query.to_string(),
    attributes: Some(workspace_tool_attributes(
      "search_files",
      workspace,
      [("query".to_string(), query.to_string())],
    )),
  });

  match search_files(Path::new(&workspace.root_path), query, 12) {
    Ok(matches) => {
      items.push(TimelineItem {
        kind: "toolResult".to_string(),
        title: "search_files result".to_string(),
        content: format_search_result(query, &matches),
        attributes: Some(workspace_tool_attributes(
          "search_files",
          workspace,
          [
            ("query".to_string(), query.to_string()),
            ("resultCount".to_string(), matches.len().to_string()),
          ],
        )),
      });
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
        title: "search_files failed".to_string(),
        content: error.to_string(),
        attributes: None,
      });
      items.push(TimelineItem {
        kind: "assistantMessage".to_string(),
        title: "Assistant".to_string(),
        content: format!(
          "Pith could not search {} yet. Try a shorter query or re-open the workspace.",
          workspace.display_name
        ),
        attributes: None,
      });
    }
  }
}
