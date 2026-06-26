use amentia_memory::MemoryNote;
use amentia_storage::StoredWorkspaceChangeRecord;
use anyhow::Result;

use crate::approval_types::PendingApproval;
use crate::runtime_context::RuntimeContext;
use crate::runtime_execution::RuntimeExecutionState;
use crate::runtime_threads::RuntimeThreadState;

impl RuntimeContext {
  pub(crate) fn persist_threads(&self) -> Result<()> {
    self.persistence_state.save_threads(&self.thread_state)
  }

  pub(crate) fn persist_runtime_state(&self) -> Result<()> {
    self
      .persistence_state
      .save_runtime_state(&self.thread_state, &self.execution_state)
  }

  pub(crate) fn persist_runtime_after_thread_delete(
    &self,
    thread_id: &str,
    thread_state: &RuntimeThreadState,
    execution_state: &RuntimeExecutionState,
  ) -> Result<()> {
    self.persistence_state.save_runtime_after_thread_delete(
      thread_state,
      execution_state,
      thread_id,
    )
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

  pub(crate) fn persist_workspace_change(
    &self,
    change: &StoredWorkspaceChangeRecord,
  ) -> Result<()> {
    self.persistence_state.save_workspace_change(change)
  }

  pub(crate) fn workspace_changes_for_thread(
    &self,
    thread_id: &str,
  ) -> Result<Vec<StoredWorkspaceChangeRecord>> {
    self
      .persistence_state
      .workspace_changes_for_thread(thread_id)
  }

  pub(crate) fn mark_workspace_change_reverted(&self, change_id: &str) -> Result<()> {
    self
      .persistence_state
      .mark_workspace_change_reverted(change_id)
  }
}
