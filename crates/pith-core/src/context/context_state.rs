use std::collections::HashMap;

use anyhow::Result;
use pith_memory::{MemoryEvent, MemoryNote};
use pith_plugin_host::{configured_plugin_install_root, configured_plugin_roots};

use crate::approval_types::PendingApproval;
use crate::plugin_catalog_state::{apply_plugin_states, load_plugin_catalog};
use crate::runtime_context::RuntimeContext;
use crate::runtime_execution::RuntimeExecutionState;
use crate::runtime_identity::RuntimeIdentity;
use crate::runtime_memory::RuntimeMemoryState;
use crate::runtime_model::RuntimeModelState;
use crate::runtime_persistence::RuntimePersistenceState;
use crate::runtime_plugins::RuntimePluginState;
use crate::runtime_sequences::RuntimeSequenceState;
use crate::runtime_threads::RuntimeThreadState;
use crate::runtime_workspace::RuntimeWorkspaceState;

impl RuntimeContext {
  pub fn new() -> Result<Self> {
    let bootstrap = RuntimePersistenceState::load_default_bootstrap()?;
    let plugin_roots = configured_plugin_roots();
    let plugin_install_root = configured_plugin_install_root();
    let plugins = apply_plugin_states(
      load_plugin_catalog(&plugin_roots)?,
      &bootstrap.plugin_states,
    );

    Ok(Self {
      identity: RuntimeIdentity::pith_runtime(),
      model_state: RuntimeModelState::new_default(true),
      persistence_state: bootstrap.persistence_state,
      memory_state: bootstrap.memory_state,
      thread_state: bootstrap.thread_state,
      workspace_state: bootstrap.workspace_state,
      plugin_state: RuntimePluginState::new(plugin_roots, plugin_install_root, plugins),
      execution_state: bootstrap.execution_state,
      sequence_state: bootstrap.sequence_state,
    })
  }

  pub fn new_in_memory() -> Self {
    let plugin_roots = configured_plugin_roots();
    let plugin_install_root = configured_plugin_install_root();
    Self {
      identity: RuntimeIdentity::pith_runtime(),
      model_state: RuntimeModelState::new_default(false),
      persistence_state: RuntimePersistenceState::in_memory(),
      memory_state: RuntimeMemoryState::new(1, vec![]),
      thread_state: RuntimeThreadState::empty(),
      workspace_state: RuntimeWorkspaceState::new(None),
      plugin_state: RuntimePluginState::new(
        plugin_roots.clone(),
        plugin_install_root,
        load_plugin_catalog(&plugin_roots).unwrap_or_default(),
      ),
      execution_state: RuntimeExecutionState::empty(),
      sequence_state: RuntimeSequenceState::new(1, 1),
    }
  }

  pub(crate) fn persist_threads(&self) -> Result<()> {
    self.persistence_state.save_threads(&self.thread_state)
  }

  pub(crate) fn persist_runtime_state(&self) -> Result<()> {
    self
      .persistence_state
      .save_runtime_state(&self.thread_state, &self.execution_state)
  }

  fn persist_memory_note(&self, note: &MemoryNote) -> Result<()> {
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

  pub(crate) fn remember(&mut self, event: MemoryEvent) -> Result<MemoryNote> {
    let note = self.memory_state.record_event(event);
    self.persist_memory_note(&note)?;
    Ok(note)
  }

  pub(crate) fn create_memory_note(
    &mut self,
    title: String,
    body: String,
    scope: String,
    source: String,
    tags: Vec<String>,
  ) -> Result<MemoryNote> {
    let note = self
      .memory_state
      .create_note(title, body, scope, source, tags);
    self.persist_memory_note(&note)?;
    Ok(note)
  }

  pub(crate) fn upsert_memory_note(
    &mut self,
    id: String,
    title: String,
    body: String,
    scope: String,
    source: String,
    tags: Vec<String>,
  ) -> Result<MemoryNote> {
    let note = self
      .memory_state
      .upsert_note(id, title, body, scope, source, tags);
    self.persist_memory_note(&note)?;
    Ok(note)
  }

  pub(crate) fn persist_plugin_enabled(&self, plugin_id: &str, enabled: bool) -> Result<()> {
    self
      .persistence_state
      .save_plugin_enabled(plugin_id, enabled)
  }

  pub(crate) fn delete_plugin_state(&self, plugin_id: &str) -> Result<()> {
    self.persistence_state.delete_plugin_state(plugin_id)
  }

  fn persisted_plugin_states(&self) -> Result<HashMap<String, bool>> {
    self.persistence_state.load_plugin_states()
  }

  pub(crate) fn refresh_plugins(&mut self) -> Result<()> {
    let plugin_states = self.persisted_plugin_states()?;
    self.plugin_state.replace_catalog(apply_plugin_states(
      load_plugin_catalog(self.plugin_state.roots())?,
      &plugin_states,
    ));
    Ok(())
  }
}

impl Default for RuntimeContext {
  fn default() -> Self {
    Self::new_in_memory()
  }
}
