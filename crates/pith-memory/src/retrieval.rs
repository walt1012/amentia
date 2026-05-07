use crate::retrieval_scoring::memory_note_score;
use crate::retrieval_text::{normalize_text, token_set};
use crate::MemoryNote;

#[derive(Debug, Clone)]
pub struct RetrievedMemoryNote {
  pub score: usize,
  pub note: MemoryNote,
}

pub fn retrieve_relevant_notes(
  notes: &[MemoryNote],
  workspace_scope: Option<&str>,
  query: &str,
  limit: usize,
) -> Vec<MemoryNote> {
  retrieve_ranked_notes(notes, workspace_scope, query, limit)
    .into_iter()
    .map(|match_result| match_result.note)
    .collect()
}

pub fn retrieve_ranked_notes(
  notes: &[MemoryNote],
  workspace_scope: Option<&str>,
  query: &str,
  limit: usize,
) -> Vec<RetrievedMemoryNote> {
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
      (score > 0).then_some(RetrievedMemoryNote { score, note })
    })
    .collect::<Vec<_>>();

  scored_notes.sort_by(|left, right| {
    right
      .score
      .cmp(&left.score)
      .then_with(|| right.note.created_at.cmp(&left.note.created_at))
      .then_with(|| left.note.id.cmp(&right.note.id))
  });

  scored_notes.into_iter().take(limit).collect()
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn retrieve_relevant_notes_prefers_workspace_and_query_matches() {
    let notes = vec![
      MemoryNote {
        id: "memory-3".to_string(),
        title: "Wrote docs/output.txt".to_string(),
        body: "Pith approved and wrote docs/output.txt in pith.".to_string(),
        scope: "pith".to_string(),
        source: "approval".to_string(),
        created_at: 30,
        tags: vec!["write".to_string(), "approval".to_string()],
      },
      MemoryNote {
        id: "memory-2".to_string(),
        title: "Opened workspace pith".to_string(),
        body: "Pith opened the workspace at /tmp/pith.".to_string(),
        scope: "pith".to_string(),
        source: "workspace".to_string(),
        created_at: 20,
        tags: vec!["workspace".to_string(), "session".to_string()],
      },
      MemoryNote {
        id: "memory-1".to_string(),
        title: "Opened workspace other".to_string(),
        body: "Pith opened the workspace at /tmp/other.".to_string(),
        scope: "other".to_string(),
        source: "workspace".to_string(),
        created_at: 10,
        tags: vec!["workspace".to_string(), "session".to_string()],
      },
    ];

    let retrieved = retrieve_relevant_notes(&notes, Some("pith"), "review docs/output.txt", 2);

    assert_eq!(retrieved.len(), 2);
    assert_eq!(retrieved[0].id, "memory-3");
    assert_eq!(retrieved[1].id, "memory-2");
  }

  #[test]
  fn ranked_retrieval_exposes_scores_for_rag_context_tracing() {
    let notes = vec![MemoryNote {
      id: "memory-1".to_string(),
      title: "Reviewed sandbox policy".to_string(),
      body: "The workspace sandbox permits local writes only.".to_string(),
      scope: "pith".to_string(),
      source: "thread".to_string(),
      created_at: 1,
      tags: vec!["sandbox".to_string()],
    }];

    let retrieved = retrieve_ranked_notes(&notes, Some("pith"), "sandbox writes", 4);

    assert_eq!(retrieved.len(), 1);
    assert_eq!(retrieved[0].note.id, "memory-1");
    assert!(retrieved[0].score > 0);
  }
}
