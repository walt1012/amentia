use std::collections::{HashMap, HashSet};

use pith_model_runtime::GenerationCancellation;
use pith_protocol::ApprovalRequest;

use super::runtime_execution_approvals::RuntimePendingApprovalState;
use super::runtime_execution_turns::RuntimeActiveTurnState;
use crate::active_turns::ActiveTurn;
use crate::approval_types::PendingApproval;

#[derive(Debug, Clone)]
pub(crate) struct RuntimeExecutionState {
  pending_approvals: RuntimePendingApprovalState,
  active_turns: RuntimeActiveTurnState,
  running_turns: HashMap<String, RunningTurnCancellation>,
  running_approvals: HashMap<String, RunningApprovalCancellation>,
  pending_running_cancellations: HashSet<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct RunningTurnCancellation {
  thread_id: String,
  cancellation: GenerationCancellation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RuntimeExecutionCounts {
  pending_approval_count: usize,
  active_turn_count: usize,
  running_approval_count: usize,
}

#[derive(Debug, Clone)]
pub(crate) struct RunningApprovalCancellation {
  thread_id: String,
  cancellation: GenerationCancellation,
}

impl RuntimeExecutionCounts {
  pub(crate) fn pending_approval_count(&self) -> usize {
    self.pending_approval_count
  }

  pub(crate) fn active_turn_count(&self) -> usize {
    self.active_turn_count
  }

  pub(crate) fn running_approval_count(&self) -> usize {
    self.running_approval_count
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
      running_turns: HashMap::new(),
      running_approvals: HashMap::new(),
      pending_running_cancellations: HashSet::new(),
    }
  }

  pub(crate) fn empty() -> Self {
    Self {
      pending_approvals: RuntimePendingApprovalState::empty(),
      active_turns: RuntimeActiveTurnState::empty(),
      running_turns: HashMap::new(),
      running_approvals: HashMap::new(),
      pending_running_cancellations: HashSet::new(),
    }
  }

  pub(crate) fn counts(&self) -> RuntimeExecutionCounts {
    RuntimeExecutionCounts {
      pending_approval_count: self.pending_approvals.count(),
      active_turn_count: self.active_turns.count() + self.running_turns.len(),
      running_approval_count: self.running_approvals.len(),
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

  pub(crate) fn insert_running_turn(
    &mut self,
    turn_id: String,
    thread_id: String,
    cancellation: GenerationCancellation,
  ) {
    self.running_turns.insert(
      turn_id,
      RunningTurnCancellation {
        thread_id,
        cancellation,
      },
    );
  }

  pub(crate) fn cancel_running_turn_for_thread(
    &mut self,
    thread_id: &str,
  ) -> Option<(String, String)> {
    let (turn_id, running_turn) = self
      .running_turns
      .iter()
      .find(|(_, turn)| turn.thread_id == thread_id)
      .map(|(turn_id, turn)| (turn_id.clone(), turn.clone()))?;
    running_turn.cancellation.cancel();
    Some((turn_id, running_turn.thread_id))
  }

  pub(crate) fn insert_running_approval(
    &mut self,
    approval_id: String,
    thread_id: String,
    cancellation: GenerationCancellation,
  ) {
    self.running_approvals.insert(
      approval_id,
      RunningApprovalCancellation {
        thread_id,
        cancellation,
      },
    );
  }

  pub(crate) fn cancel_running_approval_for_thread(&mut self, thread_id: &str) -> Option<String> {
    let running_approval = self
      .running_approvals
      .iter()
      .find(|(_, approval)| approval.thread_id == thread_id)
      .map(|(_, approval)| approval.clone())?;
    running_approval.cancellation.cancel();
    Some(running_approval.thread_id)
  }

  pub(crate) fn request_running_turn_cancel_for_thread(
    &mut self,
    thread_id: &str,
  ) -> Option<(String, String)> {
    let cancellation = self.cancel_running_turn_for_thread(thread_id).or_else(|| {
      self
        .cancel_running_approval_for_thread(thread_id)
        .map(|thread_id| ("".to_string(), thread_id))
    });
    if cancellation.is_some() {
      self.pending_running_cancellations.remove(thread_id);
    } else {
      self
        .pending_running_cancellations
        .insert(thread_id.to_string());
    }
    cancellation
  }

  pub(crate) fn take_pending_running_turn_cancel(&mut self, thread_id: &str) -> bool {
    self.pending_running_cancellations.remove(thread_id)
  }

  pub(crate) fn remove_running_turn(&mut self, turn_id: &str) {
    self.running_turns.remove(turn_id);
  }

  pub(crate) fn remove_running_approval(&mut self, approval_id: &str) {
    self.running_approvals.remove(approval_id);
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
    assert_eq!(counts.running_approval_count(), 0);
  }

  #[test]
  fn running_approval_can_be_cancelled_by_thread() {
    let mut state = RuntimeExecutionState::empty();
    let cancellation = GenerationCancellation::new();
    state.insert_running_approval(
      "approval-1".to_string(),
      "thread-1".to_string(),
      cancellation.clone(),
    );

    let cancelled = state.request_running_turn_cancel_for_thread("thread-1");

    assert_eq!(cancelled, Some(("".to_string(), "thread-1".to_string())));
    assert!(cancellation.is_cancelled());
    assert_eq!(state.counts().running_approval_count(), 1);
    state.remove_running_approval("approval-1");
    assert_eq!(state.counts().running_approval_count(), 0);
  }
}
