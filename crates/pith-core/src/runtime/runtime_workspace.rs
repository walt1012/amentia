use pith_protocol::WorkspaceSummary;

#[derive(Debug, Clone)]
pub(crate) struct RuntimeWorkspaceState {
  pub(crate) current: Option<WorkspaceSummary>,
}

impl RuntimeWorkspaceState {
  pub(crate) fn new(current: Option<WorkspaceSummary>) -> Self {
    Self { current }
  }
}
