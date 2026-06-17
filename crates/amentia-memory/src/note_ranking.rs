use crate::note_ranking_scoring::memory_note_score;
use crate::note_ranking_text::{normalize_text, token_set};
use crate::MemoryNote;

#[derive(Debug, Clone)]
pub struct RankedMemoryNote {
  pub score: usize,
  pub note: MemoryNote,
}

pub fn rank_memory_notes(
  notes: &[MemoryNote],
  workspace_scope: Option<&str>,
  query: &str,
  limit: usize,
) -> Vec<RankedMemoryNote> {
  if limit == 0 || notes.is_empty() {
    return vec![];
  }

  let query_tokens = token_set(query);
  let normalized_workspace_scope = workspace_scope.map(normalize_text);
  let mut ranked_notes = notes
    .iter()
    .cloned()
    .filter_map(|note| {
      let score = memory_note_score(&note, normalized_workspace_scope.as_deref(), &query_tokens);
      (score > 0).then_some(RankedMemoryNote { score, note })
    })
    .collect::<Vec<_>>();

  ranked_notes.sort_by(|left, right| {
    right
      .score
      .cmp(&left.score)
      .then_with(|| right.note.created_at.cmp(&left.note.created_at))
      .then_with(|| left.note.id.cmp(&right.note.id))
  });

  ranked_notes.into_iter().take(limit).collect()
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn rank_memory_notes_prefers_workspace_and_query_matches() {
    let notes = vec![
      MemoryNote {
        id: "memory-3".to_string(),
        title: "Wrote docs/output.txt".to_string(),
        body: "Amentia approved and wrote docs/output.txt in amentia.".to_string(),
        scope: "amentia".to_string(),
        source: "approval".to_string(),
        created_at: 30,
        tags: vec!["write".to_string(), "approval".to_string()],
      },
      MemoryNote {
        id: "memory-2".to_string(),
        title: "Opened workspace amentia".to_string(),
        body: "Amentia opened the workspace at /tmp/amentia.".to_string(),
        scope: "amentia".to_string(),
        source: "workspace".to_string(),
        created_at: 20,
        tags: vec!["workspace".to_string(), "session".to_string()],
      },
      MemoryNote {
        id: "memory-1".to_string(),
        title: "Opened workspace other".to_string(),
        body: "Amentia opened the workspace at /tmp/other.".to_string(),
        scope: "other".to_string(),
        source: "workspace".to_string(),
        created_at: 10,
        tags: vec!["workspace".to_string(), "session".to_string()],
      },
    ];

    let ranked = rank_memory_notes(
      &notes,
      Some("amentia"),
      "workspace review docs/output.txt",
      2,
    );

    assert_eq!(ranked.len(), 2);
    assert_eq!(ranked[0].note.id, "memory-3");
    assert_eq!(ranked[1].note.id, "memory-2");
  }

  #[test]
  fn ranked_memory_notes_expose_scores_for_memory_context_attribution() {
    let notes = vec![MemoryNote {
      id: "memory-1".to_string(),
      title: "Reviewed sandbox policy".to_string(),
      body: "The workspace sandbox permits local writes only.".to_string(),
      scope: "amentia".to_string(),
      source: "thread".to_string(),
      created_at: 1,
      tags: vec!["sandbox".to_string()],
    }];

    let ranked = rank_memory_notes(&notes, Some("amentia"), "sandbox writes", 4);

    assert_eq!(ranked.len(), 1);
    assert_eq!(ranked[0].note.id, "memory-1");
    assert!(ranked[0].score > 0);
  }

  #[test]
  fn rank_memory_notes_ignores_workspace_notes_without_query_overlap() {
    let notes = vec![MemoryNote {
      id: "memory-1".to_string(),
      title: "Opened workspace amentia".to_string(),
      body: "Amentia opened the workspace at /tmp/amentia.".to_string(),
      scope: "amentia".to_string(),
      source: "workspace".to_string(),
      created_at: 1,
      tags: vec!["workspace".to_string(), "session".to_string()],
    }];

    let ranked = rank_memory_notes(&notes, Some("amentia"), "review docs output", 4);

    assert!(ranked.is_empty());
  }
}
