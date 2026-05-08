use std::collections::HashMap;

use pith_protocol::ApprovalRequest;

use crate::approval_types::PendingApproval;

#[derive(Debug, Clone)]
pub(super) struct RuntimePendingApprovalState {
  pending_approvals: HashMap<String, PendingApproval>,
}

impl RuntimePendingApprovalState {
  pub(super) fn new(pending_approvals: HashMap<String, PendingApproval>) -> Self {
    Self { pending_approvals }
  }

  pub(super) fn empty() -> Self {
    Self::new(HashMap::new())
  }

  pub(super) fn count(&self) -> usize {
    self.pending_approvals.len()
  }

  pub(super) fn snapshot(&self, id: &str) -> Option<PendingApproval> {
    self.pending_approvals.get(id).cloned()
  }

  pub(super) fn snapshots(&self) -> Vec<PendingApproval> {
    self.pending_approvals.values().cloned().collect()
  }

  pub(super) fn requests_for_thread(&self, thread_id: &str) -> Vec<ApprovalRequest> {
    let mut approvals = self
      .pending_approvals
      .values()
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

  pub(super) fn insert(&mut self, approval: PendingApproval) {
    self.pending_approvals.insert(approval.id.clone(), approval);
  }

  pub(super) fn remove(&mut self, id: &str) -> Option<PendingApproval> {
    self.pending_approvals.remove(id)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn pending_approval(id: &str, thread_id: &str) -> PendingApproval {
    PendingApproval {
      id: id.to_string(),
      thread_id: thread_id.to_string(),
      action: "write_file".to_string(),
      title: "Write File".to_string(),
      relative_path: "src/lib.rs".to_string(),
      content: Some("content".to_string()),
      command: None,
    }
  }

  #[test]
  fn approval_requests_are_thread_scoped_and_sorted() {
    let mut state = RuntimePendingApprovalState::empty();
    state.insert(pending_approval("approval-2", "thread-1"));
    state.insert(pending_approval("approval-1", "thread-1"));
    state.insert(pending_approval("approval-3", "thread-2"));

    let approvals = state.requests_for_thread("thread-1");

    assert_eq!(
      approvals
        .iter()
        .map(|approval| approval.id.as_str())
        .collect::<Vec<_>>(),
      vec!["approval-1", "approval-2"]
    );
  }
}
