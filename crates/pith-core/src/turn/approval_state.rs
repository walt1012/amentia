use pith_protocol::ApprovalRequest;

use crate::runtime_context::RuntimeContext;

pub(crate) fn approvals_for_thread(
  context: &RuntimeContext,
  thread_id: &str,
) -> Vec<ApprovalRequest> {
  let mut approvals = context
    .execution_state
    .pending_approvals()
    .filter(|approval| approval.thread_id == thread_id)
    .map(|approval| ApprovalRequest {
      id: approval.id.clone(),
      thread_id: approval.thread_id.clone(),
      action: approval.action.clone(),
      title: approval.title.clone(),
      relative_path: approval.relative_path.clone(),
    })
    .collect::<Vec<_>>();
  approvals.sort_by(|left, right| left.id.cmp(&right.id));
  approvals
}
