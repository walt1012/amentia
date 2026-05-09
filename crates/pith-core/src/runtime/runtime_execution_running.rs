use std::collections::{HashMap, HashSet};

use pith_model_runtime::GenerationCancellation;

#[derive(Debug, Clone)]
pub(crate) struct RuntimeRunningExecutionState {
  running_turns: HashMap<String, RunningTurnCancellation>,
  running_approvals: HashMap<String, RunningApprovalCancellation>,
  pending_cancellations: HashSet<String>,
}

#[derive(Debug, Clone)]
struct RunningTurnCancellation {
  thread_id: String,
  cancellation: GenerationCancellation,
}

#[derive(Debug, Clone)]
struct RunningApprovalCancellation {
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

  pub(crate) fn request_cancel_for_thread(&mut self, thread_id: &str) -> Option<(String, String)> {
    let cancellation = self.cancel_running_turn_for_thread(thread_id).or_else(|| {
      self
        .cancel_running_approval_for_thread(thread_id)
        .map(|thread_id| ("".to_string(), thread_id))
    });
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

  pub(crate) fn remove_running_turn(&mut self, turn_id: &str) {
    self.running_turns.remove(turn_id);
  }

  pub(crate) fn remove_running_approval(&mut self, approval_id: &str) {
    self.running_approvals.remove(approval_id);
  }
}
