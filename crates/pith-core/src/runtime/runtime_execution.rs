use std::collections::HashMap;

use pith_protocol::ApprovalRequest;

use super::runtime_execution_approvals::RuntimePendingApprovalState;
use super::runtime_execution_turns::RuntimeActiveTurnState;
use crate::active_turns::ActiveTurn;
use crate::approval_types::PendingApproval;

#[derive(Debug, Clone)]
pub(crate) struct RuntimeExecutionState {
  pending_approvals: RuntimePendingApprovalState,
  active_turns: RuntimeActiveTurnState,
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
      pending_approvals: RuntimePendingApprovalState::new(pending_approvals),
      active_turns: RuntimeActiveTurnState::new(active_turns),
    }
  }

  pub(crate) fn empty() -> Self {
    Self {
      pending_approvals: RuntimePendingApprovalState::empty(),
      active_turns: RuntimeActiveTurnState::empty(),
    }
  }

  pub(crate) fn counts(&self) -> RuntimeExecutionCounts {
    RuntimeExecutionCounts {
      pending_approval_count: self.pending_approvals.count(),
      active_turn_count: self.active_turns.count(),
    }
  }

  pub(crate) fn pending_approval_snapshot(&self, id: &str) -> Option<PendingApproval> {
    self.pending_approvals.snapshot(id)
  }

  pub(crate) fn pending_approval_snapshots(&self) -> Vec<PendingApproval> {
    self.pending_approvals.snapshots()
  }

  pub(crate) fn approval_requests_for_thread(&self, thread_id: &str) -> Vec<ApprovalRequest> {
    self.pending_approvals.requests_for_thread(thread_id)
  }

  pub(crate) fn insert_pending_approval(&mut self, approval: PendingApproval) {
    self.pending_approvals.insert(approval);
  }

  pub(crate) fn remove_pending_approval(&mut self, id: &str) -> Option<PendingApproval> {
    self.pending_approvals.remove(id)
  }

  pub(crate) fn active_turn_ids(&self) -> Vec<String> {
    self.active_turns.ids()
  }

  pub(crate) fn active_turn_ids_for_thread(&self, thread_id: &str) -> Vec<String> {
    self.active_turns.ids_for_thread(thread_id)
  }

  pub(crate) fn active_turn_id_for_thread(&self, thread_id: &str) -> Option<String> {
    self.active_turns.id_for_thread(thread_id)
  }

  pub(crate) fn active_turn_snapshot(&self, id: &str) -> Option<ActiveTurn> {
    self.active_turns.snapshot(id)
  }

  pub(crate) fn update_active_turn_emitted(&mut self, id: &str, emitted_chars: usize) -> bool {
    self.active_turns.update_emitted(id, emitted_chars)
  }

  pub(crate) fn insert_active_turn(&mut self, turn: ActiveTurn) {
    self.active_turns.insert(turn);
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
}
