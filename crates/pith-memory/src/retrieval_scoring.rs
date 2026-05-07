use std::collections::HashSet;

use crate::retrieval_text::{normalize_text, token_set};
use crate::MemoryNote;

pub(crate) fn memory_note_score(
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
