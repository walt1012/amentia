use std::collections::HashMap;

use anyhow::Result;
use pith_memory::MemoryNote;
use pith_protocol::WorkspaceSummary;
use pith_storage::{RuntimeStore, StoredApprovalRecord, StoredThreadRecord};

use crate::approval_types::PendingApproval;
use crate::runtime_execution::RuntimeExecutionState;
use crate::runtime_threads::RuntimeThreadState;

#[derive(Debug, Clone)]
pub(crate) struct RuntimePersistenceState {
  store: Option<RuntimeStore>,
}

impl RuntimePersistenceState {
  pub(crate) fn new(store: Option<RuntimeStore>) -> Self {
    Self { store }
  }

  pub(crate) fn persistent(store: RuntimeStore) -> Self {
    Self::new(Some(store))
  }

  pub(crate) fn in_memory() -> Self {
    Self::new(None)
  }

  pub(crate) fn store(&self) -> Option<&RuntimeStore> {
    self.store.as_ref()
  }

  pub(crate) fn save_threads(&self, thread_state: &RuntimeThreadState) -> Result<()> {
    let Some(store) = self.store() else {
      return Ok(());
    };

    let threads = thread_state
      .iter()
      .map(|thread| StoredThreadRecord {
        summary: thread.summary().clone(),
        turn_count: thread.turn_count(),
        items: thread.items().to_vec(),
        workspace: thread.workspace_cloned(),
      })
      .collect::<Vec<_>>();

    store.save_threads(&threads)
  }

  pub(crate) fn save_pending_approvals(
    &self,
    execution_state: &RuntimeExecutionState,
  ) -> Result<()> {
    let Some(store) = self.store() else {
      return Ok(());
    };

    let approvals = execution_state
      .pending_approval_snapshots()
      .into_iter()
      .map(stored_approval_record)
      .collect::<Vec<_>>();

    store.save_pending_approvals(&approvals)
  }

  pub(crate) fn save_memory_note(&self, note: &MemoryNote) -> Result<()> {
    let Some(store) = self.store() else {
      return Ok(());
    };

    store.save_memory_note(note)
  }

  pub(crate) fn save_workspace(&self, workspace: Option<&WorkspaceSummary>) -> Result<()> {
    let Some(store) = self.store() else {
      return Ok(());
    };
    let Some(workspace) = workspace else {
      return Ok(());
    };

    store.save_workspace(workspace)
  }

  pub(crate) fn resolve_approval(&self, approval: &PendingApproval, decision: &str) -> Result<()> {
    let Some(store) = self.store() else {
      return Ok(());
    };

    store.resolve_approval(&stored_approval_record(approval.clone()), decision)
  }

  pub(crate) fn save_plugin_enabled(&self, plugin_id: &str, enabled: bool) -> Result<()> {
    let Some(store) = self.store() else {
      return Ok(());
    };

    store.save_plugin_enabled(plugin_id, enabled)
  }

  pub(crate) fn delete_plugin_state(&self, plugin_id: &str) -> Result<()> {
    let Some(store) = self.store() else {
      return Ok(());
    };

    store.delete_plugin_state(plugin_id)
  }

  pub(crate) fn load_plugin_states(&self) -> Result<HashMap<String, bool>> {
    let Some(store) = self.store() else {
      return Ok(HashMap::new());
    };

    store.load_plugin_states()
  }

  #[cfg(test)]
  pub(crate) fn set_store_for_testing(&mut self, store: RuntimeStore) {
    self.store = Some(store);
  }
}

fn stored_approval_record(approval: PendingApproval) -> StoredApprovalRecord {
  StoredApprovalRecord {
    id: approval.id,
    thread_id: approval.thread_id,
    action: approval.action,
    title: approval.title,
    relative_path: approval.relative_path,
    content: approval.content,
    command: approval.command,
  }
}
