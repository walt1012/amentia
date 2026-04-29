use std::collections::HashMap;

use pith_protocol::ApprovalRequest;

use crate::active_turns::{active_turn_id_for_thread, ActiveTurn};
use crate::approval_types::PendingApproval;

#[derive(Debug, Clone)]
pub(crate) struct RuntimeExecutionState {
  pending_approvals: HashMap<String, PendingApproval>,
  active_turns: HashMap<String, ActiveTurn>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RuntimeExecutionCounts {
  pending_approval_count: usize,
  active_turn_count: usize,
}

impl RuntimeExecutionCounts {
  pub(crate) fn pending_approval_count(&self) -> usize {
    self.pending_approval_count
  }

  pub(crate) fn active_turn_count(&self) -> usize {
    self.active_turn_count
  }
}

impl RuntimeExecutionState {
  pub(crate) fn new(
    pending_approvals: HashMap<String, PendingApproval>,
    active_turns: HashMap<String, ActiveTurn>,
  ) -> Self {
    Self {
      pending_approvals,
      active_turns,
    }
  }

  pub(crate) fn empty() -> Self {
    Self::new(HashMap::new(), HashMap::new())
  }

  pub(crate) fn counts(&self) -> RuntimeExecutionCounts {
    RuntimeExecutionCounts {
      pending_approval_count: self.pending_approvals.len(),
      active_turn_count: self.active_turns.len(),
    }
  }

  pub(crate) fn pending_approval_snapshot(&self, id: &str) -> Option<PendingApproval> {
    self.pending_approvals.get(id).cloned()
  }

  pub(crate) fn pending_approval_snapshots(&self) -> Vec<PendingApproval> {
    self.pending_approvals.values().cloned().collect()
  }

  pub(crate) fn approval_requests_for_thread(&self, thread_id: &str) -> Vec<ApprovalRequest> {
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

  pub(crate) fn insert_pending_approval(&mut self, approval: PendingApproval) {
    self.pending_approvals.insert(approval.id.clone(), approval);
  }

  pub(crate) fn remove_pending_approval(&mut self, id: &str) -> Option<PendingApproval> {
    self.pending_approvals.remove(id)
  }

  pub(crate) fn active_turn_ids(&self) -> Vec<String> {
    self.active_turns.keys().cloned().collect()
  }

  pub(crate) fn active_turn_ids_for_thread(&self, thread_id: &str) -> Vec<String> {
    self
      .active_turns
      .values()
      .filter(|turn| turn.thread_id() == thread_id)
      .map(|turn| turn.id().to_string())
      .collect()
  }

  pub(crate) fn active_turn_id_for_thread(&self, thread_id: &str) -> Option<String> {
    active_turn_id_for_thread(&self.active_turns, thread_id)
  }

  pub(crate) fn active_turn_snapshot(&self, id: &str) -> Option<ActiveTurn> {
    self.active_turns.get(id).cloned()
  }

  pub(crate) fn update_active_turn_emitted(&mut self, id: &str, emitted_chars: usize) -> bool {
    let Some(active_turn) = self.active_turns.get_mut(id) else {
      return false;
    };
    active_turn.update_emitted_chars(emitted_chars);
    true
  }

  pub(crate) fn insert_active_turn(&mut self, turn: ActiveTurn) {
    self.active_turns.insert(turn.id().to_string(), turn);
  }

  pub(crate) fn remove_active_turn(&mut self, id: &str) -> Option<ActiveTurn> {
    self.active_turns.remove(id)
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
  fn counts_hide_internal_maps() {
    let mut state = RuntimeExecutionState::empty();
    state.insert_pending_approval(pending_approval("approval-1", "thread-1"));

    let counts = state.counts();

    assert_eq!(counts.pending_approval_count(), 1);
    assert_eq!(counts.active_turn_count(), 0);
  }

  #[test]
  fn approval_requests_are_thread_scoped_and_sorted() {
    let mut state = RuntimeExecutionState::empty();
    state.insert_pending_approval(pending_approval("approval-2", "thread-1"));
    state.insert_pending_approval(pending_approval("approval-1", "thread-1"));
    state.insert_pending_approval(pending_approval("approval-3", "thread-2"));

    let approvals = state.approval_requests_for_thread("thread-1");

    assert_eq!(
      approvals
        .iter()
        .map(|approval| approval.id.as_str())
        .collect::<Vec<_>>(),
      vec!["approval-1", "approval-2"]
    );
  }
}
