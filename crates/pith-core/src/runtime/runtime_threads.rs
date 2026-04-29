use pith_protocol::WorkspaceSummary;

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

  pub(crate) fn count_for_workspace(&self, workspace: &WorkspaceSummary) -> usize {
    self.iter_for_workspace(workspace).count()
  }

  pub(crate) fn has_user_message_for_workspace(&self, workspace: &WorkspaceSummary) -> bool {
    self
      .iter_for_workspace(workspace)
      .any(|thread| thread.items.iter().any(|item| item.kind == "userMessage"))
  }

  fn iter_for_workspace<'a>(
    &'a self,
    workspace: &'a WorkspaceSummary,
  ) -> impl Iterator<Item = &'a StoredThread> + 'a {
    let root_path = workspace.root_path.as_str();
    self.iter().filter(move |thread| {
      thread_workspace_root(thread)
        .map(|thread_root_path| thread_root_path == root_path)
        .unwrap_or(false)
    })
  }
}

fn thread_workspace_root(thread: &StoredThread) -> Option<&str> {
  thread
    .workspace
    .as_ref()
    .or(thread.summary.workspace.as_ref())
    .map(|workspace| workspace.root_path.as_str())
}
