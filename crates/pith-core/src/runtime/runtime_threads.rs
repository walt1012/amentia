use crate::thread_state::StoredThread;

#[derive(Debug, Clone)]
pub(crate) struct RuntimeThreadState {
  threads: Vec<StoredThread>,
}

impl RuntimeThreadState {
  pub(crate) fn new(threads: Vec<StoredThread>) -> Self {
    Self { threads }
  }

  pub(crate) fn empty() -> Self {
    Self::new(vec![])
  }

  pub(crate) fn iter(&self) -> impl Iterator<Item = &StoredThread> {
    self.threads.iter()
  }

  pub(crate) fn iter_mut(&mut self) -> impl Iterator<Item = &mut StoredThread> {
    self.threads.iter_mut()
  }

  pub(crate) fn len(&self) -> usize {
    self.threads.len()
  }

  pub(crate) fn push(&mut self, thread: StoredThread) {
    self.threads.push(thread);
  }

  pub(crate) fn find(&self, thread_id: &str) -> Option<&StoredThread> {
    self.iter().find(|thread| thread.summary.id == thread_id)
  }

  pub(crate) fn find_mut(&mut self, thread_id: &str) -> Option<&mut StoredThread> {
    self
      .iter_mut()
      .find(|thread| thread.summary.id == thread_id)
  }
}
