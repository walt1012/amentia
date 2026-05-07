use std::collections::HashMap;

use anyhow::Result;
use pith_memory::MemoryNote;
use pith_protocol::WorkspaceSummary;
use pith_storage::RuntimeStore;

use super::runtime_persistence_bootstrap::{load_bootstrap, RuntimePersistenceBootstrap};
use super::runtime_persistence_environment::{
  save_memory_note as save_memory_note_to_store, save_workspace as save_workspace_to_store,
};
use super::runtime_persistence_execution::{
  resolve_approval as resolve_approval_in_store,
  save_runtime_state as save_runtime_state_to_store,
};
use super::runtime_persistence_plugins::{
  delete_plugin_state as delete_plugin_state_from_store,
  load_plugin_states as load_plugin_states_from_store,
  save_plugin_enabled as save_plugin_enabled_to_store,
};
use super::runtime_persistence_threads::save_threads as save_threads_to_store;
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

  pub(crate) fn load_default_bootstrap() -> Result<RuntimePersistenceBootstrap> {
    Self::load_bootstrap(RuntimeStore::new_default()?)
  }

  pub(crate) fn load_bootstrap(store: RuntimeStore) -> Result<RuntimePersistenceBootstrap> {
    load_bootstrap(store)
  }

  pub(crate) fn store(&self) -> Option<&RuntimeStore> {
    self.store.as_ref()
  }

  pub(crate) fn save_threads(&self, thread_state: &RuntimeThreadState) -> Result<()> {
    save_threads_to_store(self.store(), thread_state)
  }

  pub(crate) fn save_runtime_state(
    &self,
    thread_state: &RuntimeThreadState,
    execution_state: &RuntimeExecutionState,
  ) -> Result<()> {
    save_runtime_state_to_store(self.store(), thread_state, execution_state)
  }

  pub(crate) fn save_memory_note(&self, note: &MemoryNote) -> Result<()> {
    save_memory_note_to_store(self.store(), note)
  }

  pub(crate) fn save_workspace(&self, workspace: Option<&WorkspaceSummary>) -> Result<()> {
    save_workspace_to_store(self.store(), workspace)
  }

  pub(crate) fn resolve_approval(&self, approval: &PendingApproval, decision: &str) -> Result<()> {
    resolve_approval_in_store(self.store(), approval, decision)
  }

  pub(crate) fn save_plugin_enabled(&self, plugin_id: &str, enabled: bool) -> Result<()> {
    save_plugin_enabled_to_store(self.store(), plugin_id, enabled)
  }

  pub(crate) fn delete_plugin_state(&self, plugin_id: &str) -> Result<()> {
    delete_plugin_state_from_store(self.store(), plugin_id)
  }

  pub(crate) fn load_plugin_states(&self) -> Result<HashMap<String, bool>> {
    load_plugin_states_from_store(self.store())
  }

  #[cfg(test)]
  pub(crate) fn set_store_for_testing(&mut self, store: RuntimeStore) {
    self.store = Some(store);
  }
}
