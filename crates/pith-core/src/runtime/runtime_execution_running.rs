use std::collections::{HashMap, HashSet};

use pith_model_runtime::GenerationCancellation;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RuntimeRunningCancellation {
  turn_id: Option<String>,
  thread_id: String,
}

impl RuntimeRunningCancellation {
  fn turn(turn_id: String, thread_id: String) -> Self {
    Self {
      turn_id: Some(turn_id),
      thread_id,
    }
  }

  fn approval(thread_id: String) -> Self {
    Self {
      turn_id: None,
      thread_id,
    }
  }

  pub(crate) fn turn_id(&self) -> Option<&str> {
    self.turn_id.as_deref()
  }

  pub(crate) fn thread_id(&self) -> &str {
    &self.thread_id
  }
}

#[derive(Debug, Clone)]
pub(crate) struct RuntimeRunningExecutionState {
  running_turns: HashMap<String, RunningCancellation>,
  running_approvals: HashMap<String, RunningCancellation>,
  pending_cancellations: HashSet<String>,
}

#[derive(Debug, Clone)]
struct RunningCancellation {
  thread_id: String,
  cancellation: GenerationCancellation,
}

impl RuntimeRunningExecutionState {
  pub(crate) fn empty() -> Self {
    Self {
      running_turns: HashMap::new(),
      running_approvals: HashMap::new(),
      pending_cancellations: HashSet::new(),
    }
  }

  pub(crate) fn running_turn_count(&self) -> usize {
    self.running_turns.len()
  }

  pub(crate) fn running_approval_count(&self) -> usize {
    self.running_approvals.len()
  }

  pub(crate) fn insert_running_turn(
    &mut self,
    turn_id: String,
    thread_id: String,
    cancellation: GenerationCancellation,
  ) {
    self.running_turns.insert(
      turn_id,
      RunningCancellation {
        thread_id,
        cancellation,
      },
    );
  }

  pub(crate) fn cancel_running_turn_for_thread(
    &mut self,
    thread_id: &str,
  ) -> Option<RuntimeRunningCancellation> {
    let (turn_id, running_turn) = self
      .running_turns
      .iter()
      .find(|(_, turn)| turn.thread_id == thread_id)
      .map(|(turn_id, turn)| (turn_id.clone(), turn.clone()))?;
    running_turn.cancellation.cancel();
    Some(RuntimeRunningCancellation::turn(
      turn_id,
      running_turn.thread_id,
    ))
  }

  pub(crate) fn insert_running_approval(
    &mut self,
    approval_id: String,
    thread_id: String,
    cancellation: GenerationCancellation,
  ) {
    self.running_approvals.insert(
      approval_id,
      RunningCancellation {
        thread_id,
        cancellation,
      },
    );
  }

  pub(crate) fn cancel_running_approval_for_thread(
    &mut self,
    thread_id: &str,
  ) -> Option<RuntimeRunningCancellation> {
    let running_approval = self
      .running_approvals
      .iter()
      .find(|(_, approval)| approval.thread_id == thread_id)
      .map(|(_, approval)| approval.clone())?;
    running_approval.cancellation.cancel();
    Some(RuntimeRunningCancellation::approval(
      running_approval.thread_id,
    ))
  }

  pub(crate) fn request_cancel_for_thread(
    &mut self,
    thread_id: &str,
  ) -> Option<RuntimeRunningCancellation> {
    let cancellation = self
      .cancel_running_turn_for_thread(thread_id)
      .or_else(|| self.cancel_running_approval_for_thread(thread_id));
    if cancellation.is_some() {
      self.pending_cancellations.remove(thread_id);
    } else {
      self.pending_cancellations.insert(thread_id.to_string());
    }
    cancellation
  }

  pub(crate) fn take_pending_cancel(&mut self, thread_id: &str) -> bool {
    self.pending_cancellations.remove(thread_id)
  }

  pub(crate) fn cancel_all_running(&mut self) {
    for running_turn in self.running_turns.values() {
      running_turn.cancellation.cancel();
    }
    for running_approval in self.running_approvals.values() {
      running_approval.cancellation.cancel();
    }
    self.pending_cancellations.clear();
  }

  pub(crate) fn remove_running_turn(&mut self, turn_id: &str) {
    self.running_turns.remove(turn_id);
  }

  pub(crate) fn remove_running_approval(&mut self, approval_id: &str) {
    self.running_approvals.remove(approval_id);
  }
}
