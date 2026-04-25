use pith_memory::{retrieve_relevant_notes, MemoryNote};

pub const CONTEXT_MEMORY_NOTE_LIMIT: usize = 3;
const CONTEXT_MEMORY_CANDIDATE_LIMIT: usize = 8;
const LOCAL_CONTEXT_CHAR_BUDGET: usize = 1_200;
const MIN_NOTE_BODY_CHARS: usize = 160;

#[derive(Debug, Clone)]
pub struct ContextPack {
  pub notes: Vec<MemoryNote>,
  pub source_note_count: usize,
  pub omitted_note_count: usize,
  pub truncated_note_count: usize,
  pub estimated_char_count: usize,
  pub budget_char_count: usize,
}

impl ContextPack {
  pub fn mode(&self) -> &'static str {
    if self.notes.is_empty() {
      "empty"
    } else if self.omitted_note_count > 0 || self.truncated_note_count > 0 {
      "compacted"
    } else {
      "packed"
    }
  }
}

pub fn pack_relevant_memory_notes(
  memory_notes: &[MemoryNote],
  workspace_scope: Option<&str>,
  query: &str,
) -> ContextPack {
  let candidates = retrieve_relevant_notes(
    memory_notes,
    workspace_scope,
    query,
    CONTEXT_MEMORY_CANDIDATE_LIMIT,
  );
  let mut notes = Vec::new();
  let mut estimated_char_count = 0;
  let mut truncated_note_count = 0;

  for note in candidates.iter().take(CONTEXT_MEMORY_NOTE_LIMIT) {
    let full_note_size = estimated_note_char_count(note);
    if estimated_char_count + full_note_size <= LOCAL_CONTEXT_CHAR_BUDGET {
      notes.push(note.clone());
      estimated_char_count += full_note_size;
      continue;
    }

    let remaining_budget = LOCAL_CONTEXT_CHAR_BUDGET.saturating_sub(estimated_char_count);
    if remaining_budget < MIN_NOTE_BODY_CHARS {
      break;
    }

    let compacted_note = compact_note(note, remaining_budget);
    estimated_char_count += estimated_note_char_count(&compacted_note);
    truncated_note_count += 1;
    notes.push(compacted_note);
    break;
  }

  let omitted_note_count = candidates.len().saturating_sub(notes.len());

  ContextPack {
    notes,
    source_note_count: memory_notes.len(),
    omitted_note_count,
    truncated_note_count,
    estimated_char_count,
    budget_char_count: LOCAL_CONTEXT_CHAR_BUDGET,
  }
}

fn compact_note(note: &MemoryNote, budget: usize) -> MemoryNote {
  let fixed_size = note.title.chars().count()
    + note.scope.chars().count()
    + note.source.chars().count()
    + note
      .tags
      .iter()
      .map(|tag| tag.chars().count())
      .sum::<usize>()
    + 24;
  let body_budget = budget.saturating_sub(fixed_size).max(MIN_NOTE_BODY_CHARS);
  let mut compacted = note.clone();
  compacted.body = truncate_text(&note.body, body_budget);
  compacted
}

fn estimated_note_char_count(note: &MemoryNote) -> usize {
  note.title.chars().count()
    + note.body.chars().count()
    + note.scope.chars().count()
    + note.source.chars().count()
    + note
      .tags
      .iter()
      .map(|tag| tag.chars().count())
      .sum::<usize>()
    + 24
}

fn truncate_text(content: &str, limit: usize) -> String {
  let normalized = content.split_whitespace().collect::<Vec<_>>().join(" ");
  if normalized.chars().count() <= limit {
    return normalized;
  }

  let truncated = normalized
    .chars()
    .take(limit.saturating_sub(3))
    .collect::<String>();
  format!("{truncated}...")
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn pack_limits_relevant_memory_notes_for_local_context() {
    let notes = (0..6)
      .map(|index| MemoryNote {
        id: format!("memory-{index}"),
        title: format!("Reviewed README {index}"),
        body: "README context ".repeat(80),
        scope: "pith".to_string(),
        source: "thread".to_string(),
        created_at: index,
        tags: vec!["thread".to_string(), "summary".to_string()],
      })
      .collect::<Vec<_>>();

    let pack = pack_relevant_memory_notes(&notes, Some("pith"), "review README");

    assert!(pack.notes.len() <= CONTEXT_MEMORY_NOTE_LIMIT);
    assert!(pack.estimated_char_count <= pack.budget_char_count + MIN_NOTE_BODY_CHARS);
    assert_eq!(pack.mode(), "compacted");
  }

  #[test]
  fn pack_reports_empty_context_without_memory_notes() {
    let pack = pack_relevant_memory_notes(&[], Some("pith"), "review README");

    assert!(pack.notes.is_empty());
    assert_eq!(pack.mode(), "empty");
  }
}
