use anyhow::Result;
use pith_memory::MemoryNote;
use pith_protocol::WorkspaceSummary;
use pith_storage::RuntimeStore;

pub(super) fn save_memory_note(store: Option<&RuntimeStore>, note: &MemoryNote) -> Result<()> {
  let Some(store) = store else {
    return Ok(());
  };

  store.save_memory_note(note)
}

pub(super) fn save_workspace(
  store: Option<&RuntimeStore>,
  workspace: Option<&WorkspaceSummary>,
) -> Result<()> {
  let Some(store) = store else {
    return Ok(());
  };
  let Some(workspace) = workspace else {
    return Ok(());
  };

  store.save_workspace(workspace)
}
