use std::collections::HashMap;
use std::path::Path;

use pith_protocol::{TimelineItem, WorkspaceSummary};
use pith_tools::list_directory_with_cancellation;

use super::turn_tool_limits::LIST_DIRECTORY_RESULT_LIMIT;
use super::turn_workspace_timeline::{
  workspace_tool_failed_items, workspace_tool_result_item, workspace_tool_start_item,
};
use crate::active_turns::{start_streaming_assistant_turn, ActiveTurn};
use crate::local_responses::{
  build_plan_item, format_directory_result, summarize_directory_result,
};
use crate::plugin_permissions::{build_permission_denied_items, permission_is_granted};
use crate::request_state::PreparedTurnSnapshot;

pub(super) fn execute_list_turn(
  snapshot: &PreparedTurnSnapshot,
  workspace: &WorkspaceSummary,
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
        "Inspect the root of {} with the built-in list_directory tool.",
        workspace.display_name
      )
    } else {
      format!(
        "Check plugin permissions before inspecting the root of {}.",
        workspace.display_name
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
      "inspect the workspace",
      &workspace.display_name,
      HashMap::new(),
    ));
    return;
  }

  items.push(workspace_tool_start_item(
    "list_directory",
    ".".to_string(),
    workspace,
    [
      ("relativePath".to_string(), ".".to_string()),
      (
        "maxResults".to_string(),
        LIST_DIRECTORY_RESULT_LIMIT.to_string(),
      ),
    ],
  ));

  match list_directory_with_cancellation(
    Path::new(&workspace.root_path),
    None,
    LIST_DIRECTORY_RESULT_LIMIT,
    || snapshot.cancellation.is_cancelled(),
  ) {
    Ok(entries) => {
      items.push(workspace_tool_result_item(
        "list_directory",
        format_directory_result(&entries),
        workspace,
        [
          ("relativePath".to_string(), ".".to_string()),
          ("entryCount".to_string(), entries.len().to_string()),
          (
            "maxResults".to_string(),
            LIST_DIRECTORY_RESULT_LIMIT.to_string(),
          ),
        ],
      ));
      let (summary, summary_attributes) = summarize_directory_result(
        &snapshot.model_runtime,
        &snapshot.memory_notes,
        &snapshot.message,
        &snapshot.thread_title,
        &workspace.display_name,
        &entries,
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
      items.extend(workspace_tool_failed_items(
        "list_directory",
        error.to_string(),
        format!(
          "Pith could not inspect the root of {} yet. Re-open the workspace and try again.",
          workspace.display_name
        ),
        workspace,
        [("relativePath".to_string(), ".".to_string())],
      ));
    }
  }
}
