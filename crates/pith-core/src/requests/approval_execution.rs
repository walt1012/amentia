use std::collections::HashMap;
use std::panic::{catch_unwind, AssertUnwindSafe};

use pith_protocol::{TimelineItem, WorkspaceSummary};

use crate::approval_types::PendingApproval;
use crate::request_state::{
  ApprovalExecutionOutput, CompletedApprovalRespond, PreparedApprovalRespond,
  PreparedApprovalSnapshot,
};

use super::approval_execution_approved::execute_approved_approval;
use super::approval_execution_denied::execute_denied_approval;

pub fn execute_prepared_approval_respond(
  prepared: PreparedApprovalRespond,
) -> CompletedApprovalRespond {
  let request_id = prepared.request_id;
  let snapshot = prepared.snapshot;
  let fallback_approval = snapshot.approval.clone();
  let fallback_decision = snapshot.decision.clone();
  let fallback_workspace = snapshot.workspace.clone();
  let output = catch_unwind(AssertUnwindSafe(|| execute_approval_snapshot(snapshot)))
    .unwrap_or_else(|_| {
      build_recovered_approval_output(fallback_approval, fallback_decision, fallback_workspace)
    });

  CompletedApprovalRespond {
    request_id,
    output,
  }
}

fn execute_approval_snapshot(snapshot: PreparedApprovalSnapshot) -> ApprovalExecutionOutput {
  let PreparedApprovalSnapshot {
    approval,
    decision,
    workspace,
    model_runtime,
    cancellation,
    memory_notes,
    permission_sources,
    plugins,
  } = snapshot;

  let events = if decision == "approved" {
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

  events.into_output(approval, decision, workspace)
}

fn build_recovered_approval_output(
  approval: PendingApproval,
  decision: String,
  workspace: WorkspaceSummary,
) -> ApprovalExecutionOutput {
  ApprovalExecutionOutput {
    approval: approval.clone(),
    decision,
    workspace,
    items: vec![TimelineItem {
      kind: "warning".to_string(),
      title: "Approval Execution Recovered".to_string(),
      content: "Pith recovered after the approval action failed internally.".to_string(),
      attributes: Some(HashMap::from([
        ("approvalId".to_string(), approval.id),
        ("action".to_string(), approval.action),
      ])),
    }],
    memory_event: None,
    hook_memory_captures: vec![],
  }
}
