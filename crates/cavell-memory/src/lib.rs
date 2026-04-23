use std::collections::HashSet;
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

pub fn retrieve_relevant_notes(
  notes: &[MemoryNote],
  workspace_scope: Option<&str>,
  query: &str,
  limit: usize,
) -> Vec<MemoryNote> {
  if limit == 0 || notes.is_empty() {
    return vec![];
  }

  let query_tokens = token_set(query);
  let normalized_workspace_scope = workspace_scope.map(normalize_text);
  let mut scored_notes = notes
    .iter()
    .cloned()
    .filter_map(|note| {
      let score = memory_note_score(&note, normalized_workspace_scope.as_deref(), &query_tokens);
      (score > 0).then_some((score, note))
    })
    .collect::<Vec<_>>();

  scored_notes.sort_by(|left, right| {
    right
      .0
      .cmp(&left.0)
      .then_with(|| right.1.created_at.cmp(&left.1.created_at))
      .then_with(|| left.1.id.cmp(&right.1.id))
  });

  scored_notes
    .into_iter()
    .take(limit)
    .map(|(_, note)| note)
    .collect()
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
      format!("Cavell approved and wrote {relative_path} in {workspace_display_name}."),
      workspace_display_name,
      "approval".to_string(),
      vec!["write".to_string(), "approval".to_string()],
    ),
    MemoryEvent::ShellCommandRan {
      workspace_display_name,
      command,
    } => (
      "Ran shell command".to_string(),
      format!("Cavell approved and ran `{command}` in {workspace_display_name}."),
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

fn insert_or_replace_note(notes: &mut Vec<MemoryNote>, note: MemoryNote) {
  notes.retain(|existing| existing.id != note.id);
  notes.insert(0, note);
}

fn memory_note_score(
  note: &MemoryNote,
  workspace_scope: Option<&str>,
  query_tokens: &HashSet<String>,
) -> usize {
  let mut score = 0;
  let note_scope = normalize_text(&note.scope);
  let note_source = normalize_text(&note.source);
  let mut note_tokens = token_set(&note.title);
  note_tokens.extend(token_set(&note.body));
  note_tokens.extend(token_set(&note.scope));
  note_tokens.extend(token_set(&note.source));
  for tag in &note.tags {
    note_tokens.extend(token_set(tag));
  }

  if let Some(workspace_scope) = workspace_scope {
    if note_scope == workspace_scope {
      score += 24;
    } else if note_scope.contains(workspace_scope) || workspace_scope.contains(&note_scope) {
      score += 12;
    }
  }

  if note_source == "workspace" {
    score += 6;
  }

  let overlap_count = query_tokens.intersection(&note_tokens).count();
  score += overlap_count * 5;

  if !query_tokens.is_empty() && query_tokens.is_subset(&note_tokens) {
    score += 4;
  }

  score
}

fn token_set(content: &str) -> HashSet<String> {
  normalize_text(content)
    .split_whitespace()
    .filter(|token| !token.is_empty())
    .map(ToOwned::to_owned)
    .collect()
}

fn normalize_text(content: &str) -> String {
  content
    .chars()
    .map(|character| {
      if character.is_ascii_alphanumeric() {
        character.to_ascii_lowercase()
      } else {
        ' '
      }
    })
    .collect()
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

  #[test]
  fn manager_can_create_and_update_manual_notes() {
    let mut manager = MemoryManager::new(4);
    let mut notes = vec![];
    let created = manager.create_note(
      &mut notes,
      "Workspace preference".to_string(),
      "Prefer concise patch plans.".to_string(),
      "cavell".to_string(),
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
      "cavell".to_string(),
      "thread".to_string(),
      vec!["thread".to_string(), "summary".to_string()],
    );

    let refreshed = manager.upsert_note(
      &mut notes,
      updated.id.clone(),
      updated.title.clone(),
      "The thread reviewed README.md and wrote docs/output.txt.".to_string(),
      "cavell".to_string(),
      "thread".to_string(),
      vec!["thread".to_string(), "summary".to_string()],
    );

    assert_eq!(notes.len(), 2);
    assert_eq!(notes[0].id, "memory-thread-summary-thread-1");
    assert!(notes[0].body.contains("wrote docs/output.txt"));
    assert_eq!(refreshed.id, "memory-thread-summary-thread-1");
  }

  #[test]
  fn retrieve_relevant_notes_prefers_workspace_and_query_matches() {
    let notes = vec![
      MemoryNote {
        id: "memory-3".to_string(),
        title: "Wrote docs/output.txt".to_string(),
        body: "Cavell approved and wrote docs/output.txt in cavell.".to_string(),
        scope: "cavell".to_string(),
        source: "approval".to_string(),
        created_at: 30,
        tags: vec!["write".to_string(), "approval".to_string()],
      },
      MemoryNote {
        id: "memory-2".to_string(),
        title: "Opened workspace cavell".to_string(),
        body: "Cavell opened the workspace at /tmp/cavell.".to_string(),
        scope: "cavell".to_string(),
        source: "workspace".to_string(),
        created_at: 20,
        tags: vec!["workspace".to_string(), "session".to_string()],
      },
      MemoryNote {
        id: "memory-1".to_string(),
        title: "Opened workspace other".to_string(),
        body: "Cavell opened the workspace at /tmp/other.".to_string(),
        scope: "other".to_string(),
        source: "workspace".to_string(),
        created_at: 10,
        tags: vec!["workspace".to_string(), "session".to_string()],
      },
    ];

    let retrieved = retrieve_relevant_notes(&notes, Some("cavell"), "review docs/output.txt", 2);

    assert_eq!(retrieved.len(), 2);
    assert_eq!(retrieved[0].id, "memory-3");
    assert_eq!(retrieved[1].id, "memory-2");
  }
}
