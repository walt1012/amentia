use std::collections::HashMap;

use crate::active_turns::{active_turn_id_for_thread, ActiveTurn};

#[derive(Debug, Clone)]
pub(super) struct RuntimeActiveTurnState {
  active_turns: HashMap<String, ActiveTurn>,
}

impl RuntimeActiveTurnState {
  pub(super) fn new(active_turns: HashMap<String, ActiveTurn>) -> Self {
    Self { active_turns }
  }

  pub(super) fn empty() -> Self {
    Self::new(HashMap::new())
  }

  pub(super) fn count(&self) -> usize {
    self.active_turns.len()
  }

  pub(super) fn ids(&self) -> Vec<String> {
    self.active_turns.keys().cloned().collect()
  }

  pub(super) fn ids_for_thread(&self, thread_id: &str) -> Vec<String> {
    self
      .active_turns
      .values()
      .filter(|turn| turn.thread_id() == thread_id)
      .map(|turn| turn.id().to_string())
      .collect()
  }

  pub(super) fn id_for_thread(&self, thread_id: &str) -> Option<String> {
    active_turn_id_for_thread(&self.active_turns, thread_id)
  }

  pub(super) fn snapshot(&self, id: &str) -> Option<ActiveTurn> {
    self.active_turns.get(id).cloned()
  }

  pub(super) fn update_emitted(&mut self, id: &str, emitted_chars: usize) -> bool {
    let Some(active_turn) = self.active_turns.get_mut(id) else {
      return false;
    };
    active_turn.update_emitted_chars(emitted_chars);
    true
  }

  pub(super) fn insert(&mut self, turn: ActiveTurn) {
    self.active_turns.insert(turn.id().to_string(), turn);
  }

  pub(super) fn remove(&mut self, id: &str) -> Option<ActiveTurn> {
    self.active_turns.remove(id)
  }
}
