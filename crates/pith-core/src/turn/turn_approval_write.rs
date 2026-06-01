use std::collections::HashMap;
use std::path::Path;

use pith_protocol::{TimelineItem, WorkspaceSummary};
use pith_tools::{
  diff_preview_max_bytes, generate_diff_with_cancellation, write_file, write_file_max_bytes,
};

use super::turn_workspace_timeline::{
  workspace_diff_artifact_item, workspace_tool_result_item, workspace_tool_start_item,
  workspace_tool_warning_item,
};
use crate::approval_types::PendingApproval;
use crate::intent_inference;
use crate::local_responses::build_plan_item;
use crate::plugin_permissions::build_permission_denied_items;
use crate::request_state::PreparedTurnSnapshot;
use crate::turn::local_execution_safety::LocalChangeExecutionPolicy;
use crate::turn::turn_local_execution_block::build_local_execution_blocked_items;

pub(super) fn execute_write_turn(
  snapshot: &PreparedTurnSnapshot,
  workspace: &WorkspaceSummary,
  intent: &intent_inference::WriteIntent,
  policy: &LocalChangeExecutionPolicy,
  items: &mut Vec<TimelineItem>,
  pending_approval: &mut Option<PendingApproval>,
) {
  items.push(build_plan_item(
    &snapshot.model_runtime,
    &snapshot.memory_notes,
    &snapshot.message,
    Some(workspace),
    write_plan_summary(policy, intent, workspace),
    Some(&snapshot.cancellation),
  ));
  if snapshot.cancellation.is_cancelled() {
    items.extend(crate::turn_streaming::build_turn_cancelled_items(
      &snapshot.turn_id,
    ));
    return;
  }
  if policy.is_denied() {
    let attributes = HashMap::from([
      ("tool".to_string(), "write_file".to_string()),
      ("toolName".to_string(), "write_file".to_string()),
      ("toolKind".to_string(), "file".to_string()),
      ("actionBoundary".to_string(), "workspace".to_string()),
      ("pithAccountRequired".to_string(), "false".to_string()),
      ("relativePath".to_string(), intent.relative_path.clone()),
      (
        "localExecutionSafetyMode".to_string(),
        snapshot.local_execution_safety_mode.as_str().to_string(),
      ),
      (
        "actionApprovalPolicy".to_string(),
        policy.approval_policy_attribute().to_string(),
      ),
      (
        "blockReason".to_string(),
        policy
          .block_reason_attribute()
          .unwrap_or("unknown")
          .to_string(),
      ),
      ("retryMessage".to_string(), snapshot.message.clone()),
    ]);
    if policy.is_missing_permission_denial() {
      items.extend(build_permission_denied_items(
        &snapshot.permission_sources,
        "file.write",
        "prepare a file write",
        &workspace.display_name,
        attributes,
      ));
    } else {
      items.extend(build_local_execution_blocked_items(
        "file.write",
        "prepare a file write",
        &workspace.display_name,
        attributes,
      ));
    }
    return;
  }

  if intent.content.len() > write_file_max_bytes() {
    items.push(workspace_tool_warning_item(
      "write_file",
      "write_file rejected".to_string(),
      format!(
        "The proposed write is {} bytes, above the {} byte workspace write limit.",
        intent.content.len(),
        write_file_max_bytes()
      ),
      workspace,
      change_policy_attributes(
        snapshot,
        policy,
        [
          ("relativePath".to_string(), intent.relative_path.clone()),
          (
            "bytesRequested".to_string(),
            intent.content.len().to_string(),
          ),
          ("maxBytes".to_string(), write_file_max_bytes().to_string()),
        ],
      ),
    ));
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

  if matches!(policy, LocalChangeExecutionPolicy::AutoApproved) {
    execute_auto_approved_write(snapshot, workspace, intent, policy, items);
    return;
  }

  let LocalChangeExecutionPolicy::Ask(approval_id) = policy else {
    return;
  };

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

  items.push(workspace_tool_start_item(
    "generate_diff",
    intent.relative_path.clone(),
    workspace,
    change_policy_attributes(
      snapshot,
      policy,
      [
        ("relativePath".to_string(), intent.relative_path.clone()),
        ("maxBytes".to_string(), diff_preview_max_bytes().to_string()),
      ],
    ),
  ));
  match generate_diff_with_cancellation(
    Path::new(&workspace.root_path),
    &intent.relative_path,
    &intent.content,
    || snapshot.cancellation.is_cancelled(),
  ) {
    Ok(diff) => {
      items.push(workspace_diff_artifact_item(
        diff,
        workspace,
        change_policy_attributes(
          snapshot,
          policy,
          [
            ("action".to_string(), "write_file".to_string()),
            ("relativePath".to_string(), intent.relative_path.clone()),
            ("maxBytes".to_string(), diff_preview_max_bytes().to_string()),
          ],
        ),
      ));
    }
    Err(error) => {
      if snapshot.cancellation.is_cancelled() {
        items.extend(crate::turn_streaming::build_turn_cancelled_items(
          &snapshot.turn_id,
        ));
        return;
      }
      items.push(workspace_tool_warning_item(
        "generate_diff",
        "generate_diff failed".to_string(),
        error.to_string(),
        workspace,
        change_policy_attributes(
          snapshot,
          policy,
          [("relativePath".to_string(), intent.relative_path.clone())],
        ),
      ));
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
      (
        "localExecutionSafetyMode".to_string(),
        snapshot.local_execution_safety_mode.as_str().to_string(),
      ),
      (
        "actionApprovalPolicy".to_string(),
        policy.approval_policy_attribute().to_string(),
      ),
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

fn execute_auto_approved_write(
  snapshot: &PreparedTurnSnapshot,
  workspace: &WorkspaceSummary,
  intent: &intent_inference::WriteIntent,
  policy: &LocalChangeExecutionPolicy,
  items: &mut Vec<TimelineItem>,
) {
  items.push(workspace_tool_start_item(
    "generate_diff",
    intent.relative_path.clone(),
    workspace,
    change_policy_attributes(
      snapshot,
      policy,
      [
        ("relativePath".to_string(), intent.relative_path.clone()),
        ("maxBytes".to_string(), diff_preview_max_bytes().to_string()),
      ],
    ),
  ));
  match generate_diff_with_cancellation(
    Path::new(&workspace.root_path),
    &intent.relative_path,
    &intent.content,
    || snapshot.cancellation.is_cancelled(),
  ) {
    Ok(diff) => items.push(workspace_diff_artifact_item(
      diff,
      workspace,
      change_policy_attributes(
        snapshot,
        policy,
        [
          ("action".to_string(), "write_file".to_string()),
          ("relativePath".to_string(), intent.relative_path.clone()),
          ("maxBytes".to_string(), diff_preview_max_bytes().to_string()),
        ],
      ),
    )),
    Err(error) => {
      if snapshot.cancellation.is_cancelled() {
        items.extend(crate::turn_streaming::build_turn_cancelled_items(
          &snapshot.turn_id,
        ));
        return;
      }
      items.push(workspace_tool_warning_item(
        "generate_diff",
        "generate_diff failed".to_string(),
        error.to_string(),
        workspace,
        change_policy_attributes(
          snapshot,
          policy,
          [("relativePath".to_string(), intent.relative_path.clone())],
        ),
      ));
    }
  }
  if snapshot.cancellation.is_cancelled() {
    items.extend(crate::turn_streaming::build_turn_cancelled_items(
      &snapshot.turn_id,
    ));
    return;
  }

  match write_file(
    Path::new(&workspace.root_path),
    &intent.relative_path,
    &intent.content,
  ) {
    Ok(relative_path) => {
      items.push(workspace_tool_result_item(
        "write_file",
        format!("Wrote {} bytes to {}.", intent.content.len(), relative_path),
        workspace,
        change_policy_attributes(
          snapshot,
          policy,
          [
            ("relativePath".to_string(), relative_path.clone()),
            ("bytesWritten".to_string(), intent.content.len().to_string()),
            ("maxBytes".to_string(), write_file_max_bytes().to_string()),
          ],
        ),
      ));
      items.push(TimelineItem {
        kind: "assistantMessage".to_string(),
        title: "Assistant".to_string(),
        content: format!(
          "Pith wrote {} in {} using approved workspace execution.",
          relative_path, workspace.display_name
        ),
        attributes: Some(HashMap::from([
          ("responseRole".to_string(), "actionHandoff".to_string()),
          ("handoffKind".to_string(), "autoApprovedWrite".to_string()),
          ("action".to_string(), "write_file".to_string()),
          ("relativePath".to_string(), relative_path),
          (
            "localExecutionSafetyMode".to_string(),
            snapshot.local_execution_safety_mode.as_str().to_string(),
          ),
          (
            "actionApprovalPolicy".to_string(),
            policy.approval_policy_attribute().to_string(),
          ),
        ])),
      });
    }
    Err(error) => items.push(workspace_tool_warning_item(
      "write_file",
      "write_file failed".to_string(),
      error.to_string(),
      workspace,
      change_policy_attributes(
        snapshot,
        policy,
        [
          ("relativePath".to_string(), intent.relative_path.clone()),
          ("maxBytes".to_string(), write_file_max_bytes().to_string()),
        ],
      ),
    )),
  }
}

fn write_plan_summary(
  policy: &LocalChangeExecutionPolicy,
  intent: &intent_inference::WriteIntent,
  workspace: &WorkspaceSummary,
) -> String {
  match policy {
    LocalChangeExecutionPolicy::Ask(_) => format!(
      "Request approval before writing {} in {}.",
      intent.relative_path, workspace.display_name
    ),
    LocalChangeExecutionPolicy::AutoApproved => format!(
      "Write {} in {} using approved workspace execution.",
      intent.relative_path, workspace.display_name
    ),
    LocalChangeExecutionPolicy::Denied(_) => format!(
      "Check local execution mode and plugin permissions before writing {} in {}.",
      intent.relative_path, workspace.display_name
    ),
  }
}

fn change_policy_attributes(
  snapshot: &PreparedTurnSnapshot,
  policy: &LocalChangeExecutionPolicy,
  extra: impl IntoIterator<Item = (String, String)>,
) -> Vec<(String, String)> {
  let mut attributes = vec![
    ("actionBoundary".to_string(), "workspace".to_string()),
    ("pithAccountRequired".to_string(), "false".to_string()),
    (
      "localExecutionSafetyMode".to_string(),
      snapshot.local_execution_safety_mode.as_str().to_string(),
    ),
    (
      "actionApprovalPolicy".to_string(),
      policy.approval_policy_attribute().to_string(),
    ),
  ];
  attributes.extend(extra);
  attributes
}
