use std::collections::HashMap;
use std::path::Path;

use pith_protocol::{TimelineItem, WorkspaceSummary};
use pith_tools::shell_sandbox_summary;

use crate::approval_types::PendingApproval;
use crate::local_responses::build_plan_item;
use crate::plugin_permissions::build_permission_denied_items;
use crate::request_state::PreparedTurnSnapshot;

pub(super) fn execute_shell_turn(
  snapshot: &PreparedTurnSnapshot,
  workspace: &WorkspaceSummary,
  command: &str,
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
        "Request approval before running a shell command in {}.",
        workspace.display_name
      )
    } else {
      format!(
        "Check plugin permissions before running a shell command in {}.",
        workspace.display_name
      )
    },
  ));
  let Some(approval_id) = approval_id else {
    items.extend(build_permission_denied_items(
      &snapshot.permission_sources,
      "shell.exec",
      "run a shell command",
      &workspace.display_name,
      HashMap::from([("command".to_string(), command.to_string())]),
    ));
    return;
  };

  let sandbox = shell_sandbox_summary(Path::new(&workspace.root_path));
  let approval = PendingApproval {
    id: approval_id.clone(),
    thread_id: snapshot.thread_id.clone(),
    action: "run_shell".to_string(),
    title: "Run Shell Command".to_string(),
    relative_path: ".".to_string(),
    content: None,
    command: Some(command.to_string()),
  };
  *pending_approval = Some(approval.clone());

  items.push(TimelineItem {
    kind: "approvalRequested".to_string(),
    title: "Approval Requested".to_string(),
    content: format!(
      "Pith wants to run this shell command in {}:\n{}\n\n{}",
      workspace.display_name,
      command,
      sandbox.display_line()
    ),
    attributes: Some({
      let mut attributes = sandbox.attributes();
      attributes.extend(HashMap::from([
        ("approvalId".to_string(), approval.id.clone()),
        ("action".to_string(), approval.action.clone()),
        ("command".to_string(), command.to_string()),
      ]));
      attributes
    }),
  });
  items.push(TimelineItem {
    kind: "assistantMessage".to_string(),
    title: "Assistant".to_string(),
    content: "Pith is waiting for your approval before running the shell command.".to_string(),
    attributes: None,
  });
}
