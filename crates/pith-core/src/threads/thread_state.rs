use pith_protocol::{ThreadSummary, TimelineItem, WorkspaceSummary};

#[derive(Debug, Clone)]
pub(crate) struct StoredThread {
  pub(crate) summary: ThreadSummary,
  pub(crate) turn_count: usize,
  pub(crate) items: Vec<TimelineItem>,
  pub(crate) workspace: Option<WorkspaceSummary>,
}
