use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryNote {
  pub id: String,
  pub title: String,
  pub body: String,
  pub scope: String,
  pub source: String,
  pub created_at: i64,
  pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryStatus {
  pub note_count: usize,
  pub latest_title: Option<String>,
  pub summary: String,
}

#[derive(Debug, Clone)]
pub enum MemoryEvent {
  WorkspaceOpened {
    display_name: String,
    root_path: String,
  },
  FileWritten {
    workspace_display_name: String,
    relative_path: String,
  },
  ShellCommandRan {
    workspace_display_name: String,
    command: String,
  },
  ApprovalDenied {
    title: String,
    action: String,
  },
}
