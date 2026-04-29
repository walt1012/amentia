use pith_memory::{MemoryEvent, MemoryManager, MemoryNote, MemoryStatus};

#[derive(Debug, Clone)]
pub(crate) struct RuntimeMemoryNoteDraft {
  pub(crate) title: String,
  pub(crate) body: String,
  pub(crate) scope: String,
  pub(crate) source: String,
  pub(crate) tags: Vec<String>,
}

impl RuntimeMemoryNoteDraft {
  pub(crate) fn new(
    title: String,
    body: String,
    scope: String,
    source: String,
    tags: Vec<String>,
  ) -> Self {
    Self {
      title,
      body,
      scope,
      source,
      tags,
    }
  }
}

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

  pub(crate) fn snapshot_notes(&self) -> Vec<MemoryNote> {
    self.notes.clone()
  }

  pub(crate) fn recent_notes(&self, limit: usize) -> Vec<MemoryNote> {
    self.notes.iter().take(limit).cloned().collect()
  }

  #[cfg(test)]
  pub(crate) fn latest_note(&self) -> Option<&MemoryNote> {
    self.notes.first()
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

  pub(crate) fn create_note(&mut self, draft: RuntimeMemoryNoteDraft) -> MemoryNote {
    self.manager.create_note(
      &mut self.notes,
      draft.title,
      draft.body,
      draft.scope,
      draft.source,
      draft.tags,
    )
  }

  pub(crate) fn upsert_note(
    &mut self,
    id: String,
    draft: RuntimeMemoryNoteDraft,
  ) -> MemoryNote {
    self.manager.upsert_note(
      &mut self.notes,
      id,
      draft.title,
      draft.body,
      draft.scope,
      draft.source,
      draft.tags,
    )
  }
}
