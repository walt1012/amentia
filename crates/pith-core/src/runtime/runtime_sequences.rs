#[derive(Debug, Clone)]
pub(crate) struct RuntimeSequenceState {
  next_thread_number: usize,
  next_approval_number: usize,
}

impl RuntimeSequenceState {
  pub(crate) fn new(next_thread_number: usize, next_approval_number: usize) -> Self {
    Self {
      next_thread_number,
      next_approval_number,
    }
  }

  pub(crate) fn next_thread_id(&mut self) -> String {
    let thread_id = format!("thread-{}", self.next_thread_number);
    self.next_thread_number += 1;
    thread_id
  }

  pub(crate) fn next_approval_id(&mut self) -> String {
    let approval_id = format!("approval-{}", self.next_approval_number);
    self.next_approval_number += 1;
    approval_id
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn sequence_state_reserves_runtime_ids_in_order() {
    let mut sequences = RuntimeSequenceState::new(3, 7);

    assert_eq!(sequences.next_thread_id(), "thread-3");
    assert_eq!(sequences.next_thread_id(), "thread-4");
    assert_eq!(sequences.next_approval_id(), "approval-7");
    assert_eq!(sequences.next_approval_id(), "approval-8");
  }
}
