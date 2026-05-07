use pith_memory::MemoryNote;

#[derive(Debug, Clone)]
pub struct ContextPack {
  pub notes: Vec<MemoryNote>,
  pub retrieval_scores: Vec<usize>,
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
