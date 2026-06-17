use std::time::{SystemTime, UNIX_EPOCH};

use crate::{MemoryEvent, MemoryNote, MemoryStatus};

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

  pub fn record_event(&mut self, notes: &mut Vec<MemoryNote>, event: MemoryEvent) -> MemoryNote {
    let (title, body, scope, source, tags) = memory_note_parts(event);
    self.create_note(notes, title, body, scope, source, tags)
  }

  pub fn create_note(
    &mut self,
    notes: &mut Vec<MemoryNote>,
    title: String,
    body: String,
    scope: String,
    source: String,
    tags: Vec<String>,
  ) -> MemoryNote {
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
    insert_or_replace_note(notes, note.clone());
    note
  }

  pub fn upsert_note(
    &mut self,
    notes: &mut Vec<MemoryNote>,
    id: String,
    title: String,
    body: String,
    scope: String,
    source: String,
    tags: Vec<String>,
  ) -> MemoryNote {
    let note = MemoryNote {
      id,
      title,
      body,
      scope,
      source,
      created_at: current_timestamp(),
      tags,
    };
    insert_or_replace_note(notes, note.clone());
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
      format!("Amentia opened the workspace at {root_path}."),
      display_name,
      "workspace".to_string(),
      vec!["workspace".to_string(), "session".to_string()],
    ),
    MemoryEvent::FileWritten {
      workspace_display_name,
      relative_path,
    } => (
      format!("Wrote {relative_path}"),
      format!("Amentia approved and wrote {relative_path} in {workspace_display_name}."),
      workspace_display_name,
      "approval".to_string(),
      vec!["write".to_string(), "approval".to_string()],
    ),
    MemoryEvent::ShellCommandRan {
      workspace_display_name,
      command,
    } => (
      "Ran shell command".to_string(),
      format!("Amentia approved and ran `{command}` in {workspace_display_name}."),
      workspace_display_name,
      "approval".to_string(),
      vec!["shell".to_string(), "approval".to_string()],
    ),
    MemoryEvent::ApprovalDenied { title, action } => (
      format!("Denied {action}"),
      format!("Amentia denied the pending action: {title}."),
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

fn insert_or_replace_note(notes: &mut Vec<MemoryNote>, note: MemoryNote) {
  notes.retain(|existing| existing.id != note.id);
  notes.insert(0, note);
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
        display_name: "amentia".to_string(),
        root_path: "/tmp/amentia".to_string(),
      },
    );

    assert_eq!(note.id, "memory-1");
    assert_eq!(notes.len(), 1);
    assert_eq!(notes[0].title, "Opened workspace amentia");
  }

  #[test]
  fn manager_can_create_and_update_manual_notes() {
    let mut manager = MemoryManager::new(4);
    let mut notes = vec![];
    let created = manager.create_note(
      &mut notes,
      "Workspace preference".to_string(),
      "Prefer concise patch plans.".to_string(),
      "amentia".to_string(),
      "user".to_string(),
      vec!["workspace".to_string(), "user".to_string()],
    );

    assert_eq!(created.id, "memory-4");
    assert_eq!(notes.len(), 1);

    let updated = manager.upsert_note(
      &mut notes,
      "memory-thread-summary-thread-1".to_string(),
      "Thread summary: Thread 1".to_string(),
      "The thread reviewed README.md.".to_string(),
      "amentia".to_string(),
      "thread".to_string(),
      vec!["thread".to_string(), "summary".to_string()],
    );

    let refreshed = manager.upsert_note(
      &mut notes,
      updated.id.clone(),
      updated.title.clone(),
      "The thread reviewed README.md and wrote docs/output.txt.".to_string(),
      "amentia".to_string(),
      "thread".to_string(),
      vec!["thread".to_string(), "summary".to_string()],
    );

    assert_eq!(notes.len(), 2);
    assert_eq!(notes[0].id, "memory-thread-summary-thread-1");
    assert!(notes[0].body.contains("wrote docs/output.txt"));
    assert_eq!(refreshed.id, "memory-thread-summary-thread-1");
  }
}
