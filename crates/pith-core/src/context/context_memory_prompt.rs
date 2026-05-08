use std::collections::HashMap;

use pith_memory::MemoryNote;

use super::memory_context_types::MemoryContextPack;
use crate::text_utils::truncate_text;

const MEMORY_PROMPT_NOTE_BODY_CHARS: usize = 360;

pub fn format_memory_context_prompt(memory_context: &MemoryContextPack) -> String {
  let header = format!(
    "Memory context: mode={}, notes={}/{}, stored={}, omitted={}, truncated={}, chars={}/{}, window={}t.",
    memory_context.mode(),
    memory_context.notes.len(),
    memory_context.candidate_note_count,
    memory_context.source_note_count,
    memory_context.omitted_note_count,
    memory_context.truncated_note_count,
    memory_context.estimated_char_count,
    memory_context.budget_char_count,
    memory_context.context_window_tokens
  );
  format!(
    "{}\n{}",
    header,
    format_memory_prompt(&memory_context.notes)
  )
}

pub fn merge_memory_context_attributes(
  attributes: &mut HashMap<String, String>,
  memory_context: &MemoryContextPack,
) {
  merge_memory_attributes(attributes, &memory_context.notes);
  attributes.insert(
    "memoryContextMode".to_string(),
    memory_context.mode().to_string(),
  );
  attributes.insert(
    "memoryContextWindowTokens".to_string(),
    memory_context.context_window_tokens.to_string(),
  );
  attributes.insert(
    "memoryContextSourceNoteCount".to_string(),
    memory_context.source_note_count.to_string(),
  );
  attributes.insert(
    "memoryContextCandidateNoteCount".to_string(),
    memory_context.candidate_note_count.to_string(),
  );
  attributes.insert(
    "memoryContextOmittedNoteCount".to_string(),
    memory_context.omitted_note_count.to_string(),
  );
  attributes.insert(
    "memoryContextTruncatedNoteCount".to_string(),
    memory_context.truncated_note_count.to_string(),
  );
  attributes.insert(
    "memoryContextEstimatedChars".to_string(),
    memory_context.estimated_char_count.to_string(),
  );
  attributes.insert(
    "memoryContextBudgetChars".to_string(),
    memory_context.budget_char_count.to_string(),
  );
  if !memory_context.memory_ranking_scores.is_empty() {
    attributes.insert(
      "memoryRankingScores".to_string(),
      memory_context
        .memory_ranking_scores
        .iter()
        .map(|score| score.to_string())
        .collect::<Vec<_>>()
        .join(", "),
    );
  }
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
  fn memory_context_prompt_uses_compact_local_model_header() {
    let memory_context = MemoryContextPack {
      notes: vec![],
      memory_ranking_scores: vec![],
      context_window_tokens: 4096,
      source_note_count: 0,
      candidate_note_count: 0,
      omitted_note_count: 0,
      truncated_note_count: 0,
      estimated_char_count: 0,
      budget_char_count: 1228,
    };

    let prompt = format_memory_context_prompt(&memory_context);

    assert!(prompt.starts_with("Memory context: mode=empty"));
    assert!(prompt.contains("window=4096t"));
    assert!(!prompt.contains("stored note(s)"));
  }

  #[test]
  fn memory_context_prompt_keeps_memory_notes_single_line_and_short() {
    let memory_context = MemoryContextPack {
      notes: vec![MemoryNote {
        id: "memory-1".to_string(),
        title: "Workspace convention".to_string(),
        body: "Prefer focused changes.\nAvoid large rewrites. ".repeat(30),
        scope: "pith".to_string(),
        source: "user".to_string(),
        created_at: 1,
        tags: vec!["user".to_string()],
      }],
      memory_ranking_scores: vec![42],
      context_window_tokens: 4096,
      source_note_count: 1,
      candidate_note_count: 1,
      omitted_note_count: 0,
      truncated_note_count: 0,
      estimated_char_count: 500,
      budget_char_count: 1228,
    };

    let prompt = format_memory_context_prompt(&memory_context);
    let note_line = prompt
      .lines()
      .find(|line| line.starts_with("- Workspace convention"))
      .expect("memory note line");

    assert!(note_line.contains("[pith/user]"));
    assert!(note_line.ends_with("..."));
    assert!(note_line.chars().count() < 430);

    let mut attributes = HashMap::new();
    merge_memory_context_attributes(&mut attributes, &memory_context);
    assert_eq!(
      attributes.get("memoryRankingScores"),
      Some(&"42".to_string())
    );
  }
}
