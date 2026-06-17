use std::collections::HashSet;

use crate::note_ranking_text::{normalize_text, token_set};
use crate::MemoryNote;

pub(crate) fn memory_note_score(
  note: &MemoryNote,
  normalized_workspace_scope: Option<&str>,
  query_tokens: &HashSet<String>,
) -> usize {
  let note_scope = normalize_text(&note.scope);
  let title = normalize_text(&note.title);
  let body = normalize_text(&note.body);
  let tags = note
    .tags
    .iter()
    .map(|tag| normalize_text(tag))
    .collect::<Vec<_>>();

  let note_tokens = token_set(&format!("{} {} {}", title, body, tags.join(" ")));
  let overlap_count = query_tokens.intersection(&note_tokens).count();
  let is_pinned = tags.iter().any(|tag| tag == "pinned" || tag == "always");

  if overlap_count == 0 && !is_pinned {
    return 0;
  }

  let mut score = overlap_count * 5;

  if let Some(workspace_scope) = normalized_workspace_scope {
    if note_scope == workspace_scope {
      score += 24;
    } else if !workspace_scope.is_empty() && note_scope.contains(workspace_scope) {
      score += 12;
    }
  }

  if note.source == "user" {
    score += 4;
  }

  score
}
