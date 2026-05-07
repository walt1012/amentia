use pith_memory::{MemoryNote, MemoryStatus};
use pith_protocol::{MemoryNoteSummary, MemoryStatusResult};

pub(crate) fn to_protocol_memory_note(note: MemoryNote) -> MemoryNoteSummary {
  MemoryNoteSummary {
    id: note.id,
    title: note.title,
    body: note.body,
    scope: note.scope,
    source: note.source,
    created_at: note.created_at,
    tags: note.tags,
  }
}

pub(crate) fn to_protocol_memory_status(status: MemoryStatus) -> MemoryStatusResult {
  MemoryStatusResult {
    note_count: status.note_count,
    latest_title: status.latest_title,
    summary: status.summary,
  }
}
