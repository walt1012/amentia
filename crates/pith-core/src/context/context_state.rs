use std::collections::HashMap;

use anyhow::Result;
use pith_memory::{MemoryEvent, MemoryNote};
use pith_plugin_host::{configured_plugin_install_root, configured_plugin_roots};
use pith_storage::{RuntimeStore, StoredThreadRecord};

use crate::approval_state::stored_approval_record;
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
use crate::thread_state::StoredThread;

impl RuntimeContext {
  pub fn new() -> Result<Self> {
    let store = RuntimeStore::new_default()?;
    let persisted_threads = store.load_threads()?;
    let persisted_workspace = store.load_workspace()?;
    let persisted_pending_approvals = store.load_pending_approvals()?;
    let persisted_memory_notes = store.load_memory_notes(128)?;
    let persisted_plugin_states = store.load_plugin_states()?;
    let plugin_roots = configured_plugin_roots();
    let plugin_install_root = configured_plugin_install_root();
    let plugins = apply_plugin_states(
      load_plugin_catalog(&plugin_roots)?,
      &persisted_plugin_states,
    );
    let next_thread_number = persisted_threads.len() + 1;
    let next_approval_number = store.next_approval_sequence()?;
    let next_memory_number = store.next_memory_sequence()?;

    Ok(Self {
      identity: RuntimeIdentity::pith_runtime(),
      model_state: RuntimeModelState::new_default(true),
      persistence_state: RuntimePersistenceState::persistent(store),
      memory_state: RuntimeMemoryState::new(next_memory_number, persisted_memory_notes),
      thread_state: RuntimeThreadState::new(
        persisted_threads
          .into_iter()
          .map(|thread| StoredThread {
            summary: thread.summary,
            turn_count: thread.turn_count,
            items: thread.items,
            workspace: thread.workspace,
          })
          .collect(),
      ),
      workspace_state: RuntimeWorkspaceState::new(persisted_workspace),
      plugin_state: RuntimePluginState::new(plugin_roots, plugin_install_root, plugins),
      execution_state: RuntimeExecutionState::new(
        persisted_pending_approvals
          .into_iter()
          .map(|approval| {
            (
              approval.id.clone(),
              PendingApproval {
                id: approval.id,
                thread_id: approval.thread_id,
                action: approval.action,
                title: approval.title,
                relative_path: approval.relative_path,
                content: approval.content,
                command: approval.command,
              },
            )
          })
          .collect(),
        HashMap::new(),
      ),
      sequences: RuntimeSequenceState::new(next_thread_number, next_approval_number),
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
      sequences: RuntimeSequenceState::new(1, 1),
    }
  }

  pub(crate) fn persist_threads(&self) -> Result<()> {
    let Some(store) = self.persistence_state.store() else {
      return Ok(());
    };

    let threads = self
      .thread_state
      .iter()
      .map(|thread| StoredThreadRecord {
        summary: thread.summary.clone(),
        turn_count: thread.turn_count,
        items: thread.items.clone(),
        workspace: thread.workspace.clone(),
      })
      .collect::<Vec<_>>();

    store.save_threads(&threads)
  }

  fn persist_pending_approvals(&self) -> Result<()> {
    let Some(store) = self.persistence_state.store() else {
      return Ok(());
    };

    let approvals = self
      .execution_state
      .pending_approvals()
      .cloned()
      .map(stored_approval_record)
      .collect::<Vec<_>>();

    store.save_pending_approvals(&approvals)
  }

  pub(crate) fn persist_runtime_state(&self) -> Result<()> {
    self.persist_threads()?;
    self.persist_pending_approvals()
  }

  fn persist_memory_note(&self, note: &MemoryNote) -> Result<()> {
    let Some(store) = self.persistence_state.store() else {
      return Ok(());
    };

    store.save_memory_note(note)
  }

  pub(crate) fn persist_workspace(&self) -> Result<()> {
    let Some(store) = self.persistence_state.store() else {
      return Ok(());
    };
    let Some(workspace) = &self.workspace_state.current else {
      return Ok(());
    };

    store.save_workspace(workspace)
  }

  pub(crate) fn persist_resolved_approval(
    &self,
    approval: &PendingApproval,
    decision: &str,
  ) -> Result<()> {
    let Some(store) = self.persistence_state.store() else {
      return Ok(());
    };

    store.resolve_approval(&stored_approval_record(approval.clone()), decision)
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
    let Some(store) = self.persistence_state.store() else {
      return Ok(());
    };

    store.save_plugin_enabled(plugin_id, enabled)
  }

  pub(crate) fn delete_plugin_state(&self, plugin_id: &str) -> Result<()> {
    let Some(store) = self.persistence_state.store() else {
      return Ok(());
    };

    store.delete_plugin_state(plugin_id)
  }

  fn persisted_plugin_states(&self) -> Result<HashMap<String, bool>> {
    let Some(store) = self.persistence_state.store() else {
      return Ok(HashMap::new());
    };

    store.load_plugin_states()
  }

  pub(crate) fn refresh_plugins(&mut self) -> Result<()> {
    let plugin_states = self.persisted_plugin_states()?;
    self.plugin_state.catalog = apply_plugin_states(
      load_plugin_catalog(&self.plugin_state.roots)?,
      &plugin_states,
    );
    Ok(())
  }
}

impl Default for RuntimeContext {
  fn default() -> Self {
    Self::new_in_memory()
  }
}
