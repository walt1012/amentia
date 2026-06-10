use anyhow::Result;
use pith_memory::MemoryNote;

use crate::approval_types::PendingApproval;
use crate::runtime_context::RuntimeContext;

impl RuntimeContext {
  pub(crate) fn persist_threads(&self) -> Result<()> {
    self.persistence_state.save_threads(&self.thread_state)
  }

  pub(crate) fn persist_runtime_state(&self) -> Result<()> {
    self
      .persistence_state
      .save_runtime_state(&self.thread_state, &self.execution_state)
  }

  pub(crate) fn persist_memory_note(&self, note: &MemoryNote) -> Result<()> {
    self.persistence_state.save_memory_note(note)
  }

  pub(crate) fn persist_workspace(&self) -> Result<()> {
    self
      .persistence_state
      .save_workspace(self.workspace_state.current())
  }

  pub(crate) fn persist_resolved_approval(
    &self,
    approval: &PendingApproval,
    decision: &str,
  ) -> Result<()> {
    self.persistence_state.resolve_approval(approval, decision)
  }

  pub(crate) fn delete_approvals_for_thread(&self, thread_id: &str) -> Result<usize> {
    self.persistence_state.delete_approvals_for_thread(thread_id)
  }
}
