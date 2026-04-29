use std::collections::HashMap;

use crate::active_turns::{active_turn_id_for_thread, ActiveTurn};
use crate::approval_types::PendingApproval;

#[derive(Debug, Clone)]
pub(crate) struct RuntimeExecutionState {
  pending_approvals: HashMap<String, PendingApproval>,
  active_turns: HashMap<String, ActiveTurn>,
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

  pub(crate) fn pending_approval_count(&self) -> usize {
    self.pending_approvals.len()
  }

  pub(crate) fn active_turn_count(&self) -> usize {
    self.active_turns.len()
  }

  pub(crate) fn pending_approvals(&self) -> impl Iterator<Item = &PendingApproval> {
    self.pending_approvals.values()
  }

  pub(crate) fn pending_approval(&self, id: &str) -> Option<&PendingApproval> {
    self.pending_approvals.get(id)
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
      .filter(|turn| turn.thread_id == thread_id)
      .map(|turn| turn.id.clone())
      .collect()
  }

  pub(crate) fn active_turn_id_for_thread(&self, thread_id: &str) -> Option<String> {
    active_turn_id_for_thread(&self.active_turns, thread_id)
  }

  pub(crate) fn active_turn(&self, id: &str) -> Option<&ActiveTurn> {
    self.active_turns.get(id)
  }

  pub(crate) fn active_turn_mut(&mut self, id: &str) -> Option<&mut ActiveTurn> {
    self.active_turns.get_mut(id)
  }

  pub(crate) fn insert_active_turn(&mut self, turn: ActiveTurn) {
    self.active_turns.insert(turn.id.clone(), turn);
  }

  pub(crate) fn remove_active_turn(&mut self, id: &str) -> Option<ActiveTurn> {
    self.active_turns.remove(id)
  }
}
