use std::collections::HashMap;
use std::path::Path;

use pith_memory::MemoryEvent;
use pith_protocol::WorkspaceSummary;
use pith_tools::{write_file, write_file_max_bytes};

use crate::approval_types::PendingApproval;
use crate::plugin_permissions::{build_permission_denied_items, permission_is_granted};
use crate::turn_tool_provenance::workspace_tool_attributes;

use super::approval_execution_events::ApprovalExecutionEvents;
use super::approval_execution_timeline::{
  assistant_item, tool_result_item, tool_start_item, warning_item,
};

pub(super) fn append_approved_write_execution(
  events: &mut ApprovalExecutionEvents,
  approval: &PendingApproval,
  workspace: &WorkspaceSummary,
  permission_sources: &HashMap<String, Vec<String>>,
) {
  if !permission_is_granted(permission_sources, "file.write") {
    events.extend_items(build_permission_denied_items(
      permission_sources,
      "file.write",
      "complete the approved file write",
      &workspace.display_name,
      HashMap::from([
        ("approvalId".to_string(), approval.id.clone()),
        ("relativePath".to_string(), approval.relative_path.clone()),
      ]),
    ));
    return;
  }

  let content = approval.content.clone().unwrap_or_default();
  events.push_item(tool_start_item(
    "write_file",
    approval.relative_path.clone(),
    Some(workspace_tool_attributes(
      "write_file",
      workspace,
      [
        ("approvalId".to_string(), approval.id.clone()),
        ("relativePath".to_string(), approval.relative_path.clone()),
        ("maxBytes".to_string(), write_file_max_bytes().to_string()),
      ],
    )),
  ));

  match write_file(
    Path::new(&workspace.root_path),
    &approval.relative_path,
    &content,
  ) {
    Ok(write_result) => append_successful_write(events, workspace, approval, write_result),
    Err(error) => events.push_item(warning_item(
      "write_file failed",
      error.to_string(),
      Some(workspace_tool_attributes(
        "write_file",
        workspace,
        [
          ("approvalId".to_string(), approval.id.clone()),
          ("relativePath".to_string(), approval.relative_path.clone()),
          ("maxBytes".to_string(), write_file_max_bytes().to_string()),
        ],
      )),
    )),
  }
}

fn append_successful_write(
  events: &mut ApprovalExecutionEvents,
  workspace: &WorkspaceSummary,
  approval: &PendingApproval,
  write_result: pith_tools::WriteFileResult,
) {
  let relative_path = write_result.relative_path.clone();
  let continuation = approved_write_continuation(&relative_path);
  events.set_memory_event(MemoryEvent::FileWritten {
    workspace_display_name: workspace.display_name.clone(),
    relative_path: relative_path.clone(),
  });
  events.push_workspace_change(pith_storage::StoredWorkspaceChangeRecord {
    id: approval.id.clone(),
    thread_id: approval.thread_id.clone(),
    approval_id: Some(approval.id.clone()),
    workspace_root_path: workspace.root_path.clone(),
    relative_path: relative_path.clone(),
    action: "write_file".to_string(),
    previous_content: write_result.previous_content,
    next_content: write_result.next_content,
    reverted_at: None,
  });
  events.push_item(tool_result_item(
    "write_file result",
    format!(
      "Wrote {} bytes to {}.",
      write_result.bytes_written, relative_path
    ),
    Some(workspace_tool_attributes(
      "write_file",
      workspace,
      [
        ("approvalId".to_string(), approval.id.clone()),
        ("relativePath".to_string(), relative_path.clone()),
        (
          "bytesWritten".to_string(),
          write_result.bytes_written.to_string(),
        ),
        ("maxBytes".to_string(), write_file_max_bytes().to_string()),
      ],
    )),
  ));
  events.push_item(assistant_item(
    format!(
      "Pith wrote {} in {} after your approval. Next: {}",
      relative_path, workspace.display_name, continuation.message
    ),
    Some(approved_write_handoff_attributes(
      workspace,
      approval,
      write_result.bytes_written,
      &relative_path,
      &continuation,
    )),
  ));
}

struct ApprovedWriteContinuation {
  kind: &'static str,
  message: String,
}

fn approved_write_handoff_attributes(
  workspace: &WorkspaceSummary,
  approval: &PendingApproval,
  bytes_written: usize,
  relative_path: &str,
  continuation: &ApprovedWriteContinuation,
) -> HashMap<String, String> {
  HashMap::from([
    ("responseRole".to_string(), "actionHandoff".to_string()),
    ("handoffKind".to_string(), "approvedWrite".to_string()),
    ("approvalId".to_string(), approval.id.clone()),
    ("action".to_string(), approval.action.clone()),
    ("relativePath".to_string(), relative_path.to_string()),
    ("bytesWritten".to_string(), bytes_written.to_string()),
    (
      "continuationKind".to_string(),
      continuation.kind.to_string(),
    ),
    (
      "continuationSuggestion".to_string(),
      continuation.message.clone(),
    ),
    (
      "workspaceDisplayName".to_string(),
      workspace.display_name.clone(),
    ),
  ])
}

fn approved_write_continuation(relative_path: &str) -> ApprovedWriteContinuation {
  let normalized = relative_path.to_ascii_lowercase();
  let kind = if normalized.contains("handoff") {
    "handoffSaved"
  } else if normalized.contains("summary") || normalized.contains("review") {
    "summarySaved"
  } else if normalized.contains("note") {
    "noteSaved"
  } else {
    "fileSaved"
  };
  let message = match kind {
    "handoffSaved" => format!(
      "review {}, then ask Pith to prepare a connector update or next-action list.",
      relative_path
    ),
    "summarySaved" => format!(
      "review {}, then ask Pith to turn it into follow-up tasks if needed.",
      relative_path
    ),
    "noteSaved" => format!(
      "review {}, then continue the thread when you want to use that context.",
      relative_path
    ),
    _ => format!(
      "review {}, then ask Pith to continue from the saved change.",
      relative_path
    ),
  };

  ApprovedWriteContinuation { kind, message }
}
