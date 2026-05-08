use pith_protocol::{ThreadSummary, TimelineItem, WorkspaceSummary};

#[derive(Debug, Clone)]
pub(crate) struct StoredThread {
  summary: ThreadSummary,
  turn_count: usize,
  items: Vec<TimelineItem>,
  workspace: Option<WorkspaceSummary>,
}

#[derive(Debug, Clone)]
pub(crate) struct PreparedThreadTurn {
  pub(crate) thread_id: String,
  pub(crate) turn_id: String,
  pub(crate) thread_title: String,
  pub(crate) workspace: Option<WorkspaceSummary>,
}

impl StoredThread {
  pub(crate) fn new(
    summary: ThreadSummary,
    turn_count: usize,
    items: Vec<TimelineItem>,
    workspace: Option<WorkspaceSummary>,
  ) -> Self {
    Self {
      summary,
      turn_count,
      items,
      workspace,
    }
  }

  pub(crate) fn id(&self) -> &str {
    &self.summary.id
  }

  pub(crate) fn title(&self) -> &str {
    &self.summary.title
  }

  pub(crate) fn summary(&self) -> &ThreadSummary {
    &self.summary
  }

  pub(crate) fn status_contains(&self, needle: &str) -> bool {
    self.summary.status.contains(needle)
  }

  pub(crate) fn status(&self) -> &str {
    &self.summary.status
  }

  pub(crate) fn turn_count(&self) -> usize {
    self.turn_count
  }

  pub(crate) fn items(&self) -> &[TimelineItem] {
    &self.items
  }

  pub(crate) fn items_mut(&mut self) -> &mut Vec<TimelineItem> {
    &mut self.items
  }

  pub(crate) fn workspace(&self) -> Option<&WorkspaceSummary> {
    self.workspace.as_ref()
  }

  pub(crate) fn workspace_cloned(&self) -> Option<WorkspaceSummary> {
    self.workspace.clone()
  }

  pub(crate) fn bind_workspace_if_missing(&mut self, workspace: Option<WorkspaceSummary>) {
    if self.workspace.is_none() {
      self.workspace = workspace;
      self.summary.workspace = self.workspace.clone();
    }
  }

  pub(crate) fn begin_turn(
    &mut self,
    current_workspace: Option<WorkspaceSummary>,
  ) -> PreparedThreadTurn {
    self.turn_count += 1;
    let turn_count = self.turn_count;
    self.bind_workspace_if_missing(current_workspace);
    let workspace = self.workspace.clone();
    self.summary.status = match &workspace {
      Some(workspace) => format!("{turn_count} turn(s) in {}", workspace.display_name),
      None => format!("{turn_count} turn(s)"),
    };

    PreparedThreadTurn {
      thread_id: self.summary.id.clone(),
      turn_id: format!("{}-turn-{turn_count}", self.summary.id),
      thread_title: self.summary.title.clone(),
      workspace,
    }
  }

  pub(crate) fn begin_plugin_command(
    &mut self,
    workspace: Option<WorkspaceSummary>,
  ) -> PreparedThreadTurn {
    self.turn_count += 1;
    let turn_count = self.turn_count;
    self.bind_workspace_if_missing(workspace);
    let workspace = self.workspace.clone();
    self.summary.status = match &workspace {
      Some(workspace) => format!(
        "{turn_count} plugin command(s) in {}",
        workspace.display_name
      ),
      None => format!("{turn_count} plugin command(s)"),
    };

    PreparedThreadTurn {
      thread_id: self.summary.id.clone(),
      turn_id: format!("{}-turn-{turn_count}", self.summary.id),
      thread_title: self.summary.title.clone(),
      workspace,
    }
  }

  pub(crate) fn mark_resolving_approval(&mut self, approval_id: &str) {
    self.summary.status = format!("Resolving approval {approval_id}");
  }

  pub(crate) fn mark_streaming(&mut self) {
    self.summary.status = "Streaming assistant response".to_string();
  }

  pub(crate) fn mark_streaming_progress(&mut self, progress_label: String) {
    self.summary.status = format!("Streaming assistant response ({progress_label})");
  }

  pub(crate) fn mark_ready(&mut self) {
    self.summary.status = "Ready".to_string();
  }

  pub(crate) fn mark_cancelled(&mut self) {
    self.summary.status = "Turn cancelled".to_string();
  }

  pub(crate) fn append_items(&mut self, items: Vec<TimelineItem>) {
    self.items.extend(items);
  }

  pub(crate) fn push_item(&mut self, item: TimelineItem) {
    self.items.push(item);
  }

  pub(crate) fn snapshot(&self) -> (ThreadSummary, Vec<TimelineItem>) {
    (self.summary.clone(), self.items.clone())
  }
}
