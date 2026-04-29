use pith_protocol::ApprovalRequest;
use pith_storage::StoredApprovalRecord;

use crate::approval_types::PendingApproval;
use crate::runtime_context::RuntimeContext;

pub(crate) fn stored_approval_record(approval: PendingApproval) -> StoredApprovalRecord {
  StoredApprovalRecord {
    id: approval.id,
    thread_id: approval.thread_id,
    action: approval.action,
    title: approval.title,
    relative_path: approval.relative_path,
    content: approval.content,
    command: approval.command,
  }
}

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
