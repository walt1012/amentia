use anyhow::Result;
use amentia_memory::MemoryNote;
use amentia_protocol::WorkspaceSummary;
use amentia_storage::RuntimeStore;

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
