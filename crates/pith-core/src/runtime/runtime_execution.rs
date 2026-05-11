use std::collections::HashMap;

use pith_model_runtime::GenerationCancellation;
use pith_protocol::ApprovalRequest;

use super::runtime_execution_approvals::RuntimePendingApprovalState;
use super::runtime_execution_running::{RuntimeRunningCancellation, RuntimeRunningExecutionState};
use super::runtime_execution_turns::RuntimeActiveTurnState;
use crate::active_turns::ActiveTurn;
use crate::approval_types::PendingApproval;

#[derive(Debug, Clone)]
pub(crate) struct RuntimeExecutionState {
  pending_approvals: RuntimePendingApprovalState,
  active_turns: RuntimeActiveTurnState,
  running: RuntimeRunningExecutionState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RuntimeExecutionCounts {
  pending_approval_count: usize,
  active_turn_count: usize,
  running_turn_count: usize,
  running_approval_count: usize,
}

impl RuntimeExecutionCounts {
  pub(crate) fn pending_approval_count(&self) -> usize {
    self.pending_approval_count
  }

  pub(crate) fn active_turn_count(&self) -> usize {
    self.active_turn_count
  }

  pub(crate) fn running_turn_count(&self) -> usize {
    self.running_turn_count
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
      running: RuntimeRunningExecutionState::empty(),
    }
  }

  pub(crate) fn empty() -> Self {
    Self {
      pending_approvals: RuntimePendingApprovalState::empty(),
      active_turns: RuntimeActiveTurnState::empty(),
      running: RuntimeRunningExecutionState::empty(),
    }
  }

  pub(crate) fn counts(&self) -> RuntimeExecutionCounts {
    RuntimeExecutionCounts {
      pending_approval_count: self.pending_approvals.count(),
      active_turn_count: self.active_turns.count(),
      running_turn_count: self.running.running_turn_count(),
      running_approval_count: self.running.running_approval_count(),
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
    self
      .running
      .insert_running_turn(turn_id, thread_id, cancellation);
  }

  pub(crate) fn insert_running_approval(
    &mut self,
    approval_id: String,
    thread_id: String,
    cancellation: GenerationCancellation,
  ) {
    self
      .running
      .insert_running_approval(approval_id, thread_id, cancellation);
  }

  pub(crate) fn request_running_cancel_for_thread(
    &mut self,
    thread_id: &str,
  ) -> Option<RuntimeRunningCancellation> {
    self.running.request_cancel_for_thread(thread_id)
  }

  pub(crate) fn take_pending_running_cancel(&mut self, thread_id: &str) -> bool {
    self.running.take_pending_cancel(thread_id)
  }

  pub(crate) fn cancel_running_work(&mut self) {
    self.running.cancel_all_running();
  }

  pub(crate) fn remove_running_turn(&mut self, turn_id: &str) {
    self.running.remove_running_turn(turn_id);
  }

  pub(crate) fn remove_running_approval(&mut self, approval_id: &str) {
    self.running.remove_running_approval(approval_id);
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

    let cancelled = state.request_running_cancel_for_thread("thread-1");

    let cancelled = cancelled.expect("running approval cancelled");
    assert_eq!(cancelled.turn_id(), None);
    assert_eq!(cancelled.thread_id(), "thread-1");
    assert!(cancellation.is_cancelled());
    assert_eq!(state.counts().running_approval_count(), 1);
    state.remove_running_approval("approval-1");
    assert_eq!(state.counts().running_approval_count(), 0);
  }

  #[test]
  fn running_cancel_request_is_remembered_until_work_starts() {
    let mut state = RuntimeExecutionState::empty();

    let cancelled = state.request_running_cancel_for_thread("thread-1");

    assert_eq!(cancelled, None);
    assert!(state.take_pending_running_cancel("thread-1"));
    assert!(!state.take_pending_running_cancel("thread-1"));
  }

  #[test]
  fn running_turn_counts_separately_until_removed() {
    let mut state = RuntimeExecutionState::empty();
    state.insert_running_turn(
      "turn-1".to_string(),
      "thread-1".to_string(),
      GenerationCancellation::new(),
    );

    assert_eq!(state.counts().active_turn_count(), 0);
    assert_eq!(state.counts().running_turn_count(), 1);
    state.remove_running_turn("turn-1");
    assert_eq!(state.counts().running_turn_count(), 0);
  }

  #[test]
  fn running_turn_cancel_reports_turn_identity() {
    let mut state = RuntimeExecutionState::empty();
    let cancellation = GenerationCancellation::new();
    state.insert_running_turn(
      "turn-1".to_string(),
      "thread-1".to_string(),
      cancellation.clone(),
    );

    let cancelled = state
      .request_running_cancel_for_thread("thread-1")
      .expect("running turn cancelled");

    assert_eq!(cancelled.turn_id(), Some("turn-1"));
    assert_eq!(cancelled.thread_id(), "thread-1");
    assert!(cancellation.is_cancelled());
  }

  #[test]
  fn cancel_running_work_cancels_active_work_without_removing_records() {
    let mut state = RuntimeExecutionState::empty();
    let turn_cancellation = GenerationCancellation::new();
    let approval_cancellation = GenerationCancellation::new();
    state.insert_running_turn(
      "turn-1".to_string(),
      "thread-1".to_string(),
      turn_cancellation.clone(),
    );
    state.insert_running_approval(
      "approval-1".to_string(),
      "thread-1".to_string(),
      approval_cancellation.clone(),
    );
    assert_eq!(
      state.request_running_cancel_for_thread("queued-thread"),
      None
    );

    state.cancel_running_work();

    assert!(turn_cancellation.is_cancelled());
    assert!(approval_cancellation.is_cancelled());
    assert_eq!(state.counts().running_turn_count(), 1);
    assert_eq!(state.counts().running_approval_count(), 1);
    assert!(!state.take_pending_running_cancel("queued-thread"));
  }
}
