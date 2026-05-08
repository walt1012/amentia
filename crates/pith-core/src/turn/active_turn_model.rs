use std::time::Instant;

#[derive(Debug, Clone)]
pub(crate) struct ActiveTurn {
  id: String,
  thread_id: String,
  full_content: String,
  emitted_chars: usize,
  total_chars: usize,
  started_at: Instant,
}

impl ActiveTurn {
  pub(crate) fn new(
    id: String,
    thread_id: String,
    full_content: String,
    emitted_chars: usize,
    total_chars: usize,
  ) -> Self {
    Self {
      id,
      thread_id,
      full_content,
      emitted_chars,
      total_chars,
      started_at: Instant::now(),
    }
  }

  pub(crate) fn id(&self) -> &str {
    &self.id
  }

  pub(crate) fn thread_id(&self) -> &str {
    &self.thread_id
  }

  pub(crate) fn full_content(&self) -> &str {
    &self.full_content
  }

  pub(crate) fn emitted_chars(&self) -> usize {
    self.emitted_chars
  }

  pub(crate) fn total_chars(&self) -> usize {
    self.total_chars
  }

  pub(crate) fn update_emitted_chars(&mut self, emitted_chars: usize) {
    self.emitted_chars = emitted_chars.min(self.total_chars);
  }

  pub(crate) fn streamed_char_count(&self) -> usize {
    compute_streamed_char_count(self).min(self.total_chars)
  }

  fn started_at(&self) -> Instant {
    self.started_at
  }
}

fn compute_streamed_char_count(turn: &ActiveTurn) -> usize {
  let elapsed_steps = (turn.started_at().elapsed().as_millis() / 180) as usize;
  let base_chars = 48;
  let step_chars = 72;

  base_chars + elapsed_steps * step_chars
}
