use std::collections::HashMap;
use std::path::Path;

use pith_memory::MemoryEvent;
use pith_protocol::{TimelineItem, WorkspaceSummary};
use pith_tools::write_file;

use crate::approval_types::PendingApproval;
use crate::plugin_permissions::{build_permission_denied_items, permission_is_granted};
use crate::turn_tool_provenance::workspace_tool_attributes;

use super::approval_execution_events::ApprovalExecutionEvents;
use super::approval_execution_timeline::{assistant_item, tool_start_item, warning_item};

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
      ],
    )),
  ));

  match write_file(
    Path::new(&workspace.root_path),
    &approval.relative_path,
    &content,
  ) {
    Ok(relative_path) => {
      append_successful_write(events, workspace, approval, &content, relative_path)
    }
    Err(error) => events.push_item(warning_item("write_file failed", error.to_string())),
  }
}

fn append_successful_write(
  events: &mut ApprovalExecutionEvents,
  workspace: &WorkspaceSummary,
  approval: &PendingApproval,
  content: &str,
  relative_path: String,
) {
  events.set_memory_event(MemoryEvent::FileWritten {
    workspace_display_name: workspace.display_name.clone(),
    relative_path: relative_path.clone(),
  });
  events.push_item(TimelineItem {
    kind: "toolResult".to_string(),
    title: "write_file result".to_string(),
    content: format!("Wrote {} bytes to {}.", content.len(), relative_path),
    attributes: Some(workspace_tool_attributes(
      "write_file",
      workspace,
      [
        ("approvalId".to_string(), approval.id.clone()),
        ("relativePath".to_string(), relative_path.clone()),
        ("bytesWritten".to_string(), content.len().to_string()),
      ],
    )),
  });
  events.push_item(assistant_item(
    format!(
      "Pith wrote {} in {} after your approval.",
      relative_path, workspace.display_name
    ),
    None,
  ));
}
