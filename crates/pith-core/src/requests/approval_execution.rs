use std::collections::HashMap;
use std::panic::{catch_unwind, AssertUnwindSafe};

use pith_protocol::WorkspaceSummary;

use crate::approval_types::PendingApproval;
use crate::plugin_commands::{ensure_plugin_command_handoff, execute_plugin_command_snapshot};
use crate::request_state::{
  ApprovalExecutionOutput, CompletedApprovalRespond, PreparedApprovalRespond,
  PreparedApprovalSnapshot,
};

use super::approval_execution_approved::execute_approved_approval;
use super::approval_execution_denied::execute_denied_approval;
use super::approval_execution_timeline::warning_item;

pub fn execute_prepared_approval_respond(
  prepared: PreparedApprovalRespond,
) -> CompletedApprovalRespond {
  let request_id = prepared.request_id;
  let snapshot = prepared.snapshot;
  let fallback_approval = snapshot.approval.clone();
  let fallback_decision = snapshot.decision.clone();
  let fallback_workspace = snapshot.workspace.clone();
  let fallback_agent_context = snapshot.agent_context.clone();
  let output = catch_unwind(AssertUnwindSafe(|| execute_approval_snapshot(snapshot)))
    .unwrap_or_else(|_| {
      build_recovered_approval_output(
        fallback_approval,
        fallback_decision,
        fallback_workspace,
        fallback_agent_context,
      )
    });

  CompletedApprovalRespond { request_id, output }
}

fn execute_approval_snapshot(snapshot: PreparedApprovalSnapshot) -> ApprovalExecutionOutput {
  let PreparedApprovalSnapshot {
    approval,
    decision,
    workspace,
    agent_context,
    model_runtime,
    cancellation,
    memory_notes,
    permission_sources,
    plugins,
    approved_plugin_command,
  } = snapshot;

  let mut events = if decision == "approved" {
    execute_approved_approval(
      &approval,
      &workspace,
      &model_runtime,
      &cancellation,
      &memory_notes,
      &permission_sources,
      &plugins,
    )
  } else {
    execute_denied_approval(&approval, &workspace, &model_runtime, &memory_notes)
  };
  if decision == "approved" {
    if let Some(plugin_command) = approved_plugin_command {
      match execute_plugin_command_snapshot(plugin_command) {
        Ok(mut output) => {
          ensure_plugin_command_handoff(&mut output, "approvedPluginCommand");
          events.extend_items(output.items.clone());
          events.set_approved_plugin_command_output(output);
        }
        Err((code, message)) => {
          events.push_item(warning_item(
            "Plugin Command Failed",
            format!("{message}\n\nError code: {code}"),
            Some(HashMap::from([
              ("approvalId".to_string(), approval.id.clone()),
              ("action".to_string(), approval.action.clone()),
            ])),
          ));
        }
      }
    }
  }

  events.tag_agent_context(&agent_context);
  events.into_output(approval, decision, workspace)
}

fn build_recovered_approval_output(
  approval: PendingApproval,
  decision: String,
  workspace: WorkspaceSummary,
  agent_context: crate::requests::approval_agent_context::ApprovalAgentContext,
) -> ApprovalExecutionOutput {
  let mut items = vec![warning_item(
    "Approval Execution Recovered",
    "Pith recovered after the approval action failed internally.".to_string(),
    Some(HashMap::from([
      ("approvalId".to_string(), approval.id.clone()),
      ("action".to_string(), approval.action.clone()),
    ])),
  )];
  agent_context.tag_items(&mut items);
  ApprovalExecutionOutput {
    approval: approval.clone(),
    decision,
    workspace,
    items,
    memory_event: None,
    hook_memory_captures: vec![],
    approved_plugin_command_output: None,
    workspace_changes: vec![],
  }
}
