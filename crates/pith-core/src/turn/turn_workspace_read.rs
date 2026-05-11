use std::collections::HashMap;
use std::path::Path;

use pith_protocol::{TimelineItem, WorkspaceSummary};
use pith_tools::read_file_with_cancellation;

use super::turn_tool_limits::READ_FILE_PREVIEW_MAX_BYTES;
use super::turn_workspace_timeline::{
  workspace_tool_failed_items, workspace_tool_result_item, workspace_tool_start_item,
};
use crate::active_turns::{start_streaming_assistant_turn, ActiveTurn};
use crate::local_responses::{build_plan_item, format_file_result, summarize_file_result};
use crate::plugin_permissions::{build_permission_denied_items, permission_is_granted};
use crate::request_state::PreparedTurnSnapshot;

pub(super) fn execute_read_turn(
  snapshot: &PreparedTurnSnapshot,
  workspace: &WorkspaceSummary,
  relative_path: &str,
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
        "Inspect {} in {} with the built-in read_file tool.",
        relative_path, workspace.display_name
      )
    } else {
      format!(
        "Check plugin permissions before inspecting {} in {}.",
        relative_path, workspace.display_name
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
      "inspect a file",
      &workspace.display_name,
      HashMap::from([("relativePath".to_string(), relative_path.to_string())]),
    ));
    return;
  }

  items.push(workspace_tool_start_item(
    "read_file",
    relative_path.to_string(),
    workspace,
    [
      ("relativePath".to_string(), relative_path.to_string()),
      (
        "maxBytes".to_string(),
        READ_FILE_PREVIEW_MAX_BYTES.to_string(),
      ),
    ],
  ));

  match read_file_with_cancellation(
    Path::new(&workspace.root_path),
    relative_path,
    READ_FILE_PREVIEW_MAX_BYTES,
    || snapshot.cancellation.is_cancelled(),
  ) {
    Ok(result) => {
      items.push(workspace_tool_result_item(
        "read_file",
        format_file_result(&result),
        workspace,
        [
          ("relativePath".to_string(), relative_path.to_string()),
          (
            "maxBytes".to_string(),
            READ_FILE_PREVIEW_MAX_BYTES.to_string(),
          ),
          ("isTruncated".to_string(), result.is_truncated.to_string()),
        ],
      ));
      let (summary, summary_attributes) = summarize_file_result(
        &snapshot.model_runtime,
        &snapshot.memory_notes,
        &snapshot.message,
        &snapshot.thread_title,
        &workspace.display_name,
        &result,
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
        "read_file",
        error.to_string(),
        format!(
          "Pith could not inspect that file in {}. Try another path inside the workspace.",
          workspace.display_name
        ),
        workspace,
        [("relativePath".to_string(), relative_path.to_string())],
      ));
    }
  }
}
