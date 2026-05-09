use std::collections::HashMap;
use std::path::Path;

use pith_protocol::{TimelineItem, WorkspaceSummary};
use pith_tools::{diff_preview_max_bytes, generate_diff_with_cancellation, write_file_max_bytes};

use super::turn_tool_provenance::workspace_tool_attributes;
use crate::approval_types::PendingApproval;
use crate::intent_inference;
use crate::local_responses::build_plan_item;
use crate::plugin_permissions::build_permission_denied_items;
use crate::request_state::PreparedTurnSnapshot;

pub(super) fn execute_write_turn(
  snapshot: &PreparedTurnSnapshot,
  workspace: &WorkspaceSummary,
  intent: &intent_inference::WriteIntent,
  approval_id: &Option<String>,
  items: &mut Vec<TimelineItem>,
  pending_approval: &mut Option<PendingApproval>,
) {
  items.push(build_plan_item(
    &snapshot.model_runtime,
    &snapshot.memory_notes,
    &snapshot.message,
    Some(workspace),
    if approval_id.is_some() {
      format!(
        "Request approval before writing {} in {}.",
        intent.relative_path, workspace.display_name
      )
    } else {
      format!(
        "Check plugin permissions before writing {} in {}.",
        intent.relative_path, workspace.display_name
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
  let Some(approval_id) = approval_id else {
    items.extend(build_permission_denied_items(
      &snapshot.permission_sources,
      "file.write",
      "prepare a file write",
      &workspace.display_name,
      HashMap::from([("relativePath".to_string(), intent.relative_path.clone())]),
    ));
    return;
  };

  if intent.content.len() > write_file_max_bytes() {
    items.push(TimelineItem {
      kind: "warning".to_string(),
      title: "write_file rejected".to_string(),
      content: format!(
        "The proposed write is {} bytes, above the {} byte workspace write limit.",
        intent.content.len(),
        write_file_max_bytes()
      ),
      attributes: Some(workspace_tool_attributes(
        "write_file",
        workspace,
        [
          ("relativePath".to_string(), intent.relative_path.clone()),
          (
            "bytesRequested".to_string(),
            intent.content.len().to_string(),
          ),
          ("maxBytes".to_string(), write_file_max_bytes().to_string()),
        ],
      )),
    });
    items.push(TimelineItem {
      kind: "assistantMessage".to_string(),
      title: "Assistant".to_string(),
      content:
        "Pith did not request approval because the proposed write is too large. Split it into smaller changes."
          .to_string(),
      attributes: None,
    });
    return;
  }

  let approval = PendingApproval {
    id: approval_id.clone(),
    thread_id: snapshot.thread_id.clone(),
    action: "write_file".to_string(),
    title: format!("Write {}", intent.relative_path),
    relative_path: intent.relative_path.clone(),
    content: Some(intent.content.clone()),
    command: None,
  };
  *pending_approval = Some(approval.clone());

  items.push(TimelineItem {
    kind: "toolStart".to_string(),
    title: "generate_diff".to_string(),
    content: intent.relative_path.clone(),
    attributes: Some(workspace_tool_attributes(
      "generate_diff",
      workspace,
      [
        ("relativePath".to_string(), intent.relative_path.clone()),
        ("maxBytes".to_string(), diff_preview_max_bytes().to_string()),
      ],
    )),
  });
  match generate_diff_with_cancellation(
    Path::new(&workspace.root_path),
    &intent.relative_path,
    &intent.content,
    || snapshot.cancellation.is_cancelled(),
  ) {
    Ok(diff) => {
      items.push(TimelineItem {
        kind: "diffArtifact".to_string(),
        title: "Diff Preview".to_string(),
        content: diff,
        attributes: Some(workspace_tool_attributes(
          "generate_diff",
          workspace,
          [
            ("action".to_string(), "write_file".to_string()),
            ("relativePath".to_string(), intent.relative_path.clone()),
            ("maxBytes".to_string(), diff_preview_max_bytes().to_string()),
          ],
        )),
      });
    }
    Err(error) => {
      if snapshot.cancellation.is_cancelled() {
        items.extend(crate::turn_streaming::build_turn_cancelled_items(
          &snapshot.turn_id,
        ));
        return;
      }
      items.push(TimelineItem {
        kind: "warning".to_string(),
        title: "generate_diff failed".to_string(),
        content: error.to_string(),
        attributes: Some(workspace_tool_attributes(
          "generate_diff",
          workspace,
          [("relativePath".to_string(), intent.relative_path.clone())],
        )),
      });
    }
  }
  if snapshot.cancellation.is_cancelled() {
    items.extend(crate::turn_streaming::build_turn_cancelled_items(
      &snapshot.turn_id,
    ));
    return;
  }
  items.push(TimelineItem {
    kind: "approvalRequested".to_string(),
    title: "Approval Requested".to_string(),
    content: format!(
      "Pith wants to write {} in {}.",
      intent.relative_path, workspace.display_name
    ),
    attributes: Some(HashMap::from([
      ("approvalId".to_string(), approval.id.clone()),
      ("action".to_string(), approval.action.clone()),
      ("relativePath".to_string(), approval.relative_path.clone()),
    ])),
  });
  items.push(TimelineItem {
    kind: "assistantMessage".to_string(),
    title: "Assistant".to_string(),
    content: format!(
      "Pith prepared a write for {} and is waiting for your approval.",
      intent.relative_path
    ),
    attributes: None,
  });
}
