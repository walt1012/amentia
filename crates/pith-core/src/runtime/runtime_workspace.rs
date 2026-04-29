use pith_protocol::WorkspaceSummary;

#[derive(Debug, Clone)]
pub(crate) struct RuntimeWorkspaceState {
  current: Option<WorkspaceSummary>,
}

impl RuntimeWorkspaceState {
  pub(crate) fn new(current: Option<WorkspaceSummary>) -> Self {
    Self { current }
  }

  pub(crate) fn current(&self) -> Option<&WorkspaceSummary> {
    self.current.as_ref()
  }

  pub(crate) fn current_cloned(&self) -> Option<WorkspaceSummary> {
    self.current.clone()
  }

  pub(crate) fn is_open(&self) -> bool {
    self.current.is_some()
  }

  pub(crate) fn set_current(&mut self, workspace: WorkspaceSummary) {
    self.current = Some(workspace);
  }
}
