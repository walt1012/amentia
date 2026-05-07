use anyhow::Result;
use pith_memory::{MemoryEvent, MemoryNote};

use crate::runtime_context::RuntimeContext;
use crate::runtime_memory::RuntimeMemoryNoteDraft;

impl RuntimeContext {
  pub(crate) fn remember(&mut self, event: MemoryEvent) -> Result<MemoryNote> {
    let note = self.memory_state.record_event(event);
    self.persist_memory_note(&note)?;
    Ok(note)
  }

  pub(crate) fn create_memory_note(&mut self, draft: RuntimeMemoryNoteDraft) -> Result<MemoryNote> {
    let note = self.memory_state.create_note(draft);
    self.persist_memory_note(&note)?;
    Ok(note)
  }

  pub(crate) fn upsert_memory_note(
    &mut self,
    id: String,
    draft: RuntimeMemoryNoteDraft,
  ) -> Result<MemoryNote> {
    let note = self.memory_state.upsert_note(id, draft);
    self.persist_memory_note(&note)?;
    Ok(note)
  }
}
