use crate::request_state::{
  ApprovalExecutionOutput, CompletedApprovalRespond, PreparedApprovalRespond,
  PreparedApprovalSnapshot,
};

use super::approval_execution_approved::execute_approved_approval;
use super::approval_execution_denied::execute_denied_approval;

pub fn execute_prepared_approval_respond(
  prepared: PreparedApprovalRespond,
) -> CompletedApprovalRespond {
  CompletedApprovalRespond {
    request_id: prepared.request_id,
    output: execute_approval_snapshot(prepared.snapshot),
  }
}

fn execute_approval_snapshot(snapshot: PreparedApprovalSnapshot) -> ApprovalExecutionOutput {
  let PreparedApprovalSnapshot {
    approval,
    decision,
    workspace,
    model_runtime,
    memory_notes,
    permission_sources,
    plugins,
  } = snapshot;

  let events = if decision == "approved" {
    execute_approved_approval(
      &approval,
      &workspace,
      &model_runtime,
      &memory_notes,
      &permission_sources,
      &plugins,
    )
  } else {
    execute_denied_approval(
      &approval,
      &workspace,
      &model_runtime,
      &memory_notes,
    )
  };

  events.into_output(approval, decision, workspace)
}
