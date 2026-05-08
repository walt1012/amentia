#[derive(Debug, Clone)]
pub(crate) struct PendingApproval {
  pub(crate) id: String,
  pub(crate) thread_id: String,
  pub(crate) action: String,
  pub(crate) title: String,
  pub(crate) relative_path: String,
  pub(crate) content: Option<String>,
  pub(crate) command: Option<String>,
}
