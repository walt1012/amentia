use pith_memory::{MemoryEvent, MemoryManager, MemoryNote, MemoryStatus};

#[derive(Debug, Clone)]
pub(crate) struct RuntimeMemoryState {
  manager: MemoryManager,
  notes: Vec<MemoryNote>,
}

impl RuntimeMemoryState {
  pub(crate) fn new(next_note_number: usize, notes: Vec<MemoryNote>) -> Self {
    Self {
      manager: MemoryManager::new(next_note_number),
      notes,
    }
  }

  pub(crate) fn notes(&self) -> &[MemoryNote] {
    &self.notes
  }

  pub(crate) fn snapshot_notes(&self) -> Vec<MemoryNote> {
    self.notes.clone()
  }

  pub(crate) fn note_count(&self) -> usize {
    self.notes.len()
  }

  pub(crate) fn status(&self) -> MemoryStatus {
    self.manager.status(&self.notes)
  }

  pub(crate) fn record_event(&mut self, event: MemoryEvent) -> MemoryNote {
    self.manager.record_event(&mut self.notes, event)
  }

  pub(crate) fn create_note(
    &mut self,
    title: String,
    body: String,
    scope: String,
    source: String,
    tags: Vec<String>,
  ) -> MemoryNote {
    self
      .manager
      .create_note(&mut self.notes, title, body, scope, source, tags)
  }

  pub(crate) fn upsert_note(
    &mut self,
    id: String,
    title: String,
    body: String,
    scope: String,
    source: String,
    tags: Vec<String>,
  ) -> MemoryNote {
    self
      .manager
      .upsert_note(&mut self.notes, id, title, body, scope, source, tags)
  }
}
