use pith_protocol::ApprovalRequest;

use crate::runtime_context::RuntimeContext;

pub(crate) fn approvals_for_thread(
  context: &RuntimeContext,
  thread_id: &str,
) -> Vec<ApprovalRequest> {
  context.execution_state.approval_requests_for_thread(thread_id)
}
