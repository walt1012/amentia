use std::collections::HashMap;

use pith_memory::MemoryNote;

use super::context_pack_types::ContextPack;
use crate::text_utils::truncate_text;

const MEMORY_PROMPT_NOTE_BODY_CHARS: usize = 360;

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
