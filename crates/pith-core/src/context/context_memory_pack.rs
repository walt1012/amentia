use std::collections::HashMap;

use pith_memory::{retrieve_relevant_notes, MemoryNote};
use pith_model_runtime::LocalModelRuntime;

use crate::text_utils::truncate_text;

const DEFAULT_MODEL_CONTEXT_TOKENS: usize = 4096;
const CONTEXT_MEMORY_BUDGET_PERCENT: usize = 30;
const MIN_CONTEXT_MEMORY_CHAR_BUDGET: usize = 900;
const MAX_CONTEXT_MEMORY_CHAR_BUDGET: usize = 2400;
pub const CONTEXT_MEMORY_NOTE_LIMIT: usize = 3;
const CONTEXT_MEMORY_CANDIDATE_LIMIT: usize = 8;
const MIN_NOTE_BODY_CHARS: usize = 160;
const MEMORY_PROMPT_NOTE_BODY_CHARS: usize = 360;

#[derive(Debug, Clone)]
pub struct ContextPack {
  pub notes: Vec<MemoryNote>,
  pub context_window_tokens: usize,
  pub source_note_count: usize,
  pub candidate_note_count: usize,
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

pub fn pack_memory_context(
  model_runtime: &LocalModelRuntime,
  memory_notes: &[MemoryNote],
  workspace_scope: Option<&str>,
  query: &str,
) -> ContextPack {
  let (budget_char_count, context_window_tokens) = context_budget_for_model(model_runtime);
  pack_relevant_memory_notes(
    memory_notes,
    workspace_scope,
    query,
    budget_char_count,
    context_window_tokens,
  )
}

pub fn pack_relevant_memory_notes(
  memory_notes: &[MemoryNote],
  workspace_scope: Option<&str>,
  query: &str,
  budget_char_count: usize,
  context_window_tokens: usize,
) -> ContextPack {
  let budget_char_count = budget_char_count.max(MIN_NOTE_BODY_CHARS);
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
    if estimated_char_count + full_note_size <= budget_char_count {
      notes.push(note.clone());
      estimated_char_count += full_note_size;
      continue;
    }

    let remaining_budget = budget_char_count.saturating_sub(estimated_char_count);
    let Some(compacted_note) = compact_note(note, remaining_budget) else {
      break;
    };
    estimated_char_count += estimated_note_char_count(&compacted_note);
    truncated_note_count += 1;
    notes.push(compacted_note);
    break;
  }

  let omitted_note_count = candidates.len().saturating_sub(notes.len());

  ContextPack {
    notes,
    context_window_tokens,
    source_note_count: memory_notes.len(),
    candidate_note_count: candidates.len(),
    omitted_note_count,
    truncated_note_count,
    estimated_char_count,
    budget_char_count,
  }
}

pub fn format_context_prompt(context_pack: &ContextPack) -> String {
  let header = format!(
    "Context: mode={}, notes={}/{}, stored={}, omitted={}, truncated={}, chars={}/{}, window={}t.",
    context_pack.mode(),
    context_pack.notes.len(),
    context_pack.candidate_note_count,
    context_pack.source_note_count,
    context_pack.omitted_note_count,
    context_pack.truncated_note_count,
    context_pack.estimated_char_count,
    context_pack.budget_char_count,
    context_pack.context_window_tokens
  );
  format!("{}\n{}", header, format_memory_prompt(&context_pack.notes))
}

pub fn merge_context_pack_attributes(
  attributes: &mut HashMap<String, String>,
  context_pack: &ContextPack,
) {
  merge_memory_attributes(attributes, &context_pack.notes);
  attributes.insert("contextMode".to_string(), context_pack.mode().to_string());
  attributes.insert(
    "contextWindowTokens".to_string(),
    context_pack.context_window_tokens.to_string(),
  );
  attributes.insert(
    "contextSourceNoteCount".to_string(),
    context_pack.source_note_count.to_string(),
  );
  attributes.insert(
    "contextCandidateNoteCount".to_string(),
    context_pack.candidate_note_count.to_string(),
  );
  attributes.insert(
    "contextOmittedNoteCount".to_string(),
    context_pack.omitted_note_count.to_string(),
  );
  attributes.insert(
    "contextTruncatedNoteCount".to_string(),
    context_pack.truncated_note_count.to_string(),
  );
  attributes.insert(
    "contextEstimatedChars".to_string(),
    context_pack.estimated_char_count.to_string(),
  );
  attributes.insert(
    "contextBudgetChars".to_string(),
    context_pack.budget_char_count.to_string(),
  );
}

fn context_budget_for_model(model_runtime: &LocalModelRuntime) -> (usize, usize) {
  let health = model_runtime.health();
  let context_window_tokens = health
    .metrics
    .get("contextSize")
    .and_then(|value| value.parse::<usize>().ok())
    .filter(|value| *value > 0)
    .unwrap_or(DEFAULT_MODEL_CONTEXT_TOKENS);
  let raw_budget = context_window_tokens.saturating_mul(CONTEXT_MEMORY_BUDGET_PERCENT) / 100;
  let budget_char_count = raw_budget.clamp(
    MIN_CONTEXT_MEMORY_CHAR_BUDGET,
    MAX_CONTEXT_MEMORY_CHAR_BUDGET,
  );
  (budget_char_count, context_window_tokens)
}

fn compact_note(note: &MemoryNote, budget: usize) -> Option<MemoryNote> {
  let fixed_size = note.title.chars().count()
    + note.scope.chars().count()
    + note.source.chars().count()
    + note
      .tags
      .iter()
      .map(|tag| tag.chars().count())
      .sum::<usize>()
    + 24;
  let body_budget = budget.saturating_sub(fixed_size);
  if body_budget < MIN_NOTE_BODY_CHARS {
    return None;
  }
  let mut compacted = note.clone();
  compacted.body = truncate_text(&note.body, body_budget);
  Some(compacted)
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

fn format_memory_prompt(memory_notes: &[MemoryNote]) -> String {
  if memory_notes.is_empty() {
    return "Memory: none.".to_string();
  }

  let note_lines = memory_notes
    .iter()
    .map(|note| {
      let body = truncate_text(&note.body, MEMORY_PROMPT_NOTE_BODY_CHARS);
      format!(
        "- {} [{}/{}]: {}",
        note.title, note.scope, note.source, body
      )
    })
    .collect::<Vec<_>>()
    .join("\n");

  format!("Relevant memory notes:\n{note_lines}")
}

fn merge_memory_attributes(attributes: &mut HashMap<String, String>, memory_notes: &[MemoryNote]) {
  attributes.insert(
    "memoryNoteCount".to_string(),
    memory_notes.len().to_string(),
  );
  if memory_notes.is_empty() {
    return;
  }

  attributes.insert(
    "memoryNoteIds".to_string(),
    memory_notes
      .iter()
      .map(|note| note.id.clone())
      .collect::<Vec<_>>()
      .join(", "),
  );
  attributes.insert(
    "memoryNoteTitles".to_string(),
    memory_notes
      .iter()
      .map(|note| note.title.clone())
      .collect::<Vec<_>>()
      .join(" | "),
  );
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

    let pack = pack_relevant_memory_notes(&notes, Some("pith"), "review README", 1_200, 4096);

    assert!(pack.notes.len() <= CONTEXT_MEMORY_NOTE_LIMIT);
    assert!(pack.estimated_char_count <= pack.budget_char_count);
    assert_eq!(pack.mode(), "compacted");
    assert_eq!(pack.context_window_tokens, 4096);
    assert_eq!(pack.candidate_note_count, 6);
  }

  #[test]
  fn pack_reports_empty_context_without_memory_notes() {
    let pack = pack_relevant_memory_notes(&[], Some("pith"), "review README", 1_200, 4096);

    assert!(pack.notes.is_empty());
    assert_eq!(pack.mode(), "empty");
    assert_eq!(pack.candidate_note_count, 0);
  }

  #[test]
  fn context_prompt_uses_compact_local_model_header() {
    let context_pack = ContextPack {
      notes: vec![],
      context_window_tokens: 4096,
      source_note_count: 0,
      candidate_note_count: 0,
      omitted_note_count: 0,
      truncated_note_count: 0,
      estimated_char_count: 0,
      budget_char_count: 1228,
    };

    let prompt = format_context_prompt(&context_pack);

    assert!(prompt.starts_with("Context: mode=empty"));
    assert!(prompt.contains("window=4096t"));
    assert!(!prompt.contains("stored note(s)"));
  }

  #[test]
  fn context_prompt_keeps_memory_notes_single_line_and_short() {
    let context_pack = ContextPack {
      notes: vec![MemoryNote {
        id: "memory-1".to_string(),
        title: "Workspace convention".to_string(),
        body: "Prefer focused changes.\nAvoid large rewrites. ".repeat(30),
        scope: "pith".to_string(),
        source: "user".to_string(),
        created_at: 1,
        tags: vec!["user".to_string()],
      }],
      context_window_tokens: 4096,
      source_note_count: 1,
      candidate_note_count: 1,
      omitted_note_count: 0,
      truncated_note_count: 0,
      estimated_char_count: 500,
      budget_char_count: 1228,
    };

    let prompt = format_context_prompt(&context_pack);
    let note_line = prompt
      .lines()
      .find(|line| line.starts_with("- Workspace convention"))
      .expect("memory note line");

    assert!(note_line.contains("[pith/user]"));
    assert!(note_line.ends_with("..."));
    assert!(note_line.chars().count() < 430);
  }
}
