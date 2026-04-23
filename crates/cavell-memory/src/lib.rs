use std::time::{SystemTime, UNIX_EPOCH};

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

#[derive(Debug, Clone)]
pub struct MemoryManager {
  next_note_number: usize,
}

impl MemoryManager {
  pub fn new(next_note_number: usize) -> Self {
    Self { next_note_number }
  }

  pub fn status(&self, notes: &[MemoryNote]) -> MemoryStatus {
    let latest_title = notes.first().map(|note| note.title.clone());
    let summary = if let Some(note) = notes.first() {
      format!(
        "Built-in memory is tracking {} note(s). Latest: {}.",
        notes.len(),
        note.title
      )
    } else {
      "Built-in memory is ready but has not captured any workspace notes yet.".to_string()
    };

    MemoryStatus {
      note_count: notes.len(),
      latest_title,
      summary,
    }
  }

  pub fn record_event(
    &mut self,
    notes: &mut Vec<MemoryNote>,
    event: MemoryEvent,
  ) -> MemoryNote {
    let (title, body, scope, source, tags) = memory_note_parts(event);
    let note = MemoryNote {
      id: format!("memory-{}", self.next_note_number),
      title,
      body,
      scope,
      source,
      created_at: current_timestamp(),
      tags,
    };
    self.next_note_number += 1;
    notes.insert(0, note.clone());
    note
  }
}

fn memory_note_parts(event: MemoryEvent) -> (String, String, String, String, Vec<String>) {
  match event {
    MemoryEvent::WorkspaceOpened {
      display_name,
      root_path,
    } => (
      format!("Opened workspace {display_name}"),
      format!("Cavell opened the workspace at {root_path}."),
      display_name,
      "workspace".to_string(),
      vec!["workspace".to_string(), "session".to_string()],
    ),
    MemoryEvent::FileWritten {
      workspace_display_name,
      relative_path,
    } => (
      format!("Wrote {relative_path}"),
      format!(
        "Cavell approved and wrote {relative_path} in {workspace_display_name}."
      ),
      workspace_display_name,
      "approval".to_string(),
      vec!["write".to_string(), "approval".to_string()],
    ),
    MemoryEvent::ShellCommandRan {
      workspace_display_name,
      command,
    } => (
      "Ran shell command".to_string(),
      format!(
        "Cavell approved and ran `{command}` in {workspace_display_name}."
      ),
      workspace_display_name,
      "approval".to_string(),
      vec!["shell".to_string(), "approval".to_string()],
    ),
    MemoryEvent::ApprovalDenied { title, action } => (
      format!("Denied {action}"),
      format!("Cavell denied the pending action: {title}."),
      "global".to_string(),
      "approval".to_string(),
      vec!["approval".to_string(), "denied".to_string()],
    ),
  }
}

fn current_timestamp() -> i64 {
  SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .expect("system time")
    .as_secs() as i64
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn manager_records_workspace_notes() {
    let mut manager = MemoryManager::new(1);
    let mut notes = vec![];
    let note = manager.record_event(
      &mut notes,
      MemoryEvent::WorkspaceOpened {
        display_name: "cavell".to_string(),
        root_path: "/tmp/cavell".to_string(),
      },
    );

    assert_eq!(note.id, "memory-1");
    assert_eq!(notes.len(), 1);
    assert_eq!(notes[0].title, "Opened workspace cavell");
  }
}
