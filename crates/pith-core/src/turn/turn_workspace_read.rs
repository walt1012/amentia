use std::collections::HashMap;
use std::path::Path;

use pith_protocol::{TimelineItem, WorkspaceSummary};
use pith_tools::read_file;

use crate::active_turns::{start_streaming_assistant_turn, ActiveTurn};
use crate::local_responses::{build_plan_item, format_file_result, summarize_file_result};
use crate::plugin_permissions::{build_permission_denied_items, permission_is_granted};
use crate::request_state::PreparedTurnSnapshot;
use crate::turn_tool_provenance::workspace_tool_attributes;

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

  items.push(TimelineItem {
    kind: "toolStart".to_string(),
    title: "read_file".to_string(),
    content: relative_path.to_string(),
    attributes: Some(workspace_tool_attributes(
      "read_file",
      workspace,
      [("relativePath".to_string(), relative_path.to_string())],
    )),
  });

  match read_file(Path::new(&workspace.root_path), relative_path, 4096) {
    Ok(result) => {
      items.push(TimelineItem {
        kind: "toolResult".to_string(),
        title: "read_file result".to_string(),
        content: format_file_result(&result),
        attributes: Some(workspace_tool_attributes(
          "read_file",
          workspace,
          [("relativePath".to_string(), relative_path.to_string())],
        )),
      });
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
      items.push(TimelineItem {
        kind: "warning".to_string(),
        title: "read_file failed".to_string(),
        content: error.to_string(),
        attributes: None,
      });
      items.push(TimelineItem {
        kind: "assistantMessage".to_string(),
        title: "Assistant".to_string(),
        content: format!(
          "Pith could not inspect that file in {}. Try another path inside the workspace.",
          workspace.display_name
        ),
        attributes: None,
      });
    }
  }
}
