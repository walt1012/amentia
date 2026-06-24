use std::collections::HashMap;
use std::path::Path;

use amentia_protocol::{TimelineItem, WorkspaceSummary};
use amentia_tools::{run_shell_with_cancellation, shell_sandbox_summary};

use crate::approval_types::PendingApproval;
use crate::local_responses::{build_plan_item, format_shell_result, summarize_shell_result};
use crate::plugin_permission_denied::build_permission_denied_items;
use crate::request_state::PreparedTurnSnapshot;
use crate::turn::local_execution_safety::LocalChangeExecutionPolicy;
use crate::turn::turn_local_execution_block::build_local_execution_blocked_items;
use crate::turn::turn_tool_limits::SHELL_OUTPUT_PREVIEW_MAX_BYTES;
use crate::turn_tool_provenance::workspace_tool_attributes;

pub(super) fn execute_shell_turn(
  snapshot: &PreparedTurnSnapshot,
  workspace: &WorkspaceSummary,
  command: &str,
  policy: &LocalChangeExecutionPolicy,
  items: &mut Vec<TimelineItem>,
  pending_approval: &mut Option<PendingApproval>,
) {
  items.push(build_plan_item(
    &snapshot.model_runtime,
    &snapshot.memory_notes,
    &snapshot.plugin_skill_context,
    &snapshot.message,
    Some(workspace),
    shell_plan_summary(policy, workspace),
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
      ("tool".to_string(), "run_shell".to_string()),
      ("toolName".to_string(), "run_shell".to_string()),
      ("toolKind".to_string(), "shell".to_string()),
      ("actionBoundary".to_string(), "workspace".to_string()),
      ("amentiaAccountRequired".to_string(), "false".to_string()),
      ("command".to_string(), command.to_string()),
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
        "shell.exec",
        "run a shell command",
        &workspace.display_name,
        attributes,
      ));
    } else {
      items.extend(build_local_execution_blocked_items(
        "shell.exec",
        "run a shell command",
        &workspace.display_name,
        attributes,
      ));
    }
    return;
  }

  if matches!(policy, LocalChangeExecutionPolicy::AutoApproved) {
    execute_auto_approved_shell(snapshot, workspace, command, policy, items);
    return;
  }

  let LocalChangeExecutionPolicy::Ask(approval_id) = policy else {
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
      "Amentia wants to run this shell command in {}:\n{}\n\n{}",
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
        (
          "localExecutionSafetyMode".to_string(),
          snapshot.local_execution_safety_mode.as_str().to_string(),
        ),
        (
          "actionApprovalPolicy".to_string(),
          policy.approval_policy_attribute().to_string(),
        ),
      ]));
      attributes
    }),
  });
  items.push(TimelineItem {
    kind: "assistantMessage".to_string(),
    title: "Assistant".to_string(),
    content: "Amentia is waiting for your approval before running the shell command.".to_string(),
    attributes: None,
  });
}

fn execute_auto_approved_shell(
  snapshot: &PreparedTurnSnapshot,
  workspace: &WorkspaceSummary,
  command: &str,
  policy: &LocalChangeExecutionPolicy,
  items: &mut Vec<TimelineItem>,
) {
  items.push(TimelineItem {
    kind: "toolStart".to_string(),
    title: "run_shell".to_string(),
    content: command.to_string(),
    attributes: Some(workspace_tool_attributes(
      "run_shell",
      workspace,
      change_policy_attributes(
        snapshot,
        policy,
        [
          ("command".to_string(), command.to_string()),
          (
            "maxOutputBytes".to_string(),
            SHELL_OUTPUT_PREVIEW_MAX_BYTES.to_string(),
          ),
        ],
      ),
    )),
  });

  match run_shell_with_cancellation(
    Path::new(&workspace.root_path),
    command,
    SHELL_OUTPUT_PREVIEW_MAX_BYTES,
    || snapshot.cancellation.is_cancelled(),
  ) {
    Ok(result) => {
      let (summary, summary_attributes) = summarize_shell_result(
        &snapshot.model_runtime,
        &snapshot.memory_notes,
        &workspace.display_name,
        &result,
        Some(&snapshot.cancellation),
      );
      items.push(TimelineItem {
        kind: "toolResult".to_string(),
        title: "run_shell result".to_string(),
        content: format_shell_result(&result),
        attributes: Some({
          let mut attributes = workspace_tool_attributes(
            "run_shell",
            workspace,
            change_policy_attributes(
              snapshot,
              policy,
              [
                ("command".to_string(), command.to_string()),
                ("exitCode".to_string(), result.exit_code.to_string()),
                ("timedOut".to_string(), result.timed_out.to_string()),
                ("cancelled".to_string(), result.cancelled.to_string()),
                (
                  "maxOutputBytes".to_string(),
                  SHELL_OUTPUT_PREVIEW_MAX_BYTES.to_string(),
                ),
              ],
            ),
          );
          attributes.extend(result.sandbox.attributes());
          attributes.extend(result.output_context.attributes());
          attributes
        }),
      });
      items.push(TimelineItem {
        kind: "assistantMessage".to_string(),
        title: "Assistant".to_string(),
        content: summary,
        attributes: Some(auto_approved_shell_handoff_attributes(
          summary_attributes,
          snapshot,
          policy,
          command,
          &result,
        )),
      });
    }
    Err(error) => items.push(TimelineItem {
      kind: "warning".to_string(),
      title: "run_shell failed".to_string(),
      content: error.to_string(),
      attributes: Some(workspace_tool_attributes(
        "run_shell",
        workspace,
        change_policy_attributes(
          snapshot,
          policy,
          [("command".to_string(), command.to_string())],
        ),
      )),
    }),
  }
}

fn shell_plan_summary(policy: &LocalChangeExecutionPolicy, workspace: &WorkspaceSummary) -> String {
  match policy {
    LocalChangeExecutionPolicy::Ask(_) => format!(
      "Request approval before running a shell command in {}.",
      workspace.display_name
    ),
    LocalChangeExecutionPolicy::AutoApproved => format!(
      "Run a shell command in {} using approved workspace execution.",
      workspace.display_name
    ),
    LocalChangeExecutionPolicy::Denied(_) => format!(
      "Check action safety mode and plugin permissions before running a shell command in {}.",
      workspace.display_name
    ),
  }
}

fn auto_approved_shell_handoff_attributes(
  mut attributes: HashMap<String, String>,
  snapshot: &PreparedTurnSnapshot,
  policy: &LocalChangeExecutionPolicy,
  command: &str,
  result: &amentia_tools::ShellCommandResult,
) -> HashMap<String, String> {
  attributes.extend(HashMap::from([
    ("responseRole".to_string(), "actionHandoff".to_string()),
    ("handoffKind".to_string(), "autoApprovedShell".to_string()),
    ("action".to_string(), "run_shell".to_string()),
    ("command".to_string(), command.to_string()),
    ("exitCode".to_string(), result.exit_code.to_string()),
    ("timedOut".to_string(), result.timed_out.to_string()),
    ("cancelled".to_string(), result.cancelled.to_string()),
    (
      "localExecutionSafetyMode".to_string(),
      snapshot.local_execution_safety_mode.as_str().to_string(),
    ),
    (
      "actionApprovalPolicy".to_string(),
      policy.approval_policy_attribute().to_string(),
    ),
  ]));
  attributes
}

fn change_policy_attributes(
  snapshot: &PreparedTurnSnapshot,
  policy: &LocalChangeExecutionPolicy,
  extra: impl IntoIterator<Item = (String, String)>,
) -> Vec<(String, String)> {
  let mut attributes = vec![
    ("actionBoundary".to_string(), "workspace".to_string()),
    ("amentiaAccountRequired".to_string(), "false".to_string()),
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
