use std::collections::HashMap;

use anyhow::Result;
use pith_storage::RuntimeStore;

use super::runtime_persistence::RuntimePersistenceState;
use super::runtime_persistence_records::{pending_approval, stored_thread};
use crate::runtime_execution::RuntimeExecutionState;
use crate::runtime_memory::RuntimeMemoryState;
use crate::runtime_sequences::RuntimeSequenceState;
use crate::runtime_threads::RuntimeThreadState;
use crate::runtime_workspace::RuntimeWorkspaceState;

pub(crate) struct RuntimePersistenceBootstrap {
  pub(crate) persistence_state: RuntimePersistenceState,
  pub(crate) memory_state: RuntimeMemoryState,
  pub(crate) thread_state: RuntimeThreadState,
  pub(crate) workspace_state: RuntimeWorkspaceState,
  pub(crate) execution_state: RuntimeExecutionState,
  pub(crate) sequence_state: RuntimeSequenceState,
  pub(crate) plugin_states: HashMap<String, bool>,
}

pub(super) fn load_bootstrap(store: RuntimeStore) -> Result<RuntimePersistenceBootstrap> {
  let persisted_threads = store.load_threads()?;
  let persisted_workspace = store.load_workspace()?;
  let persisted_pending_approvals = store.load_pending_approvals()?;
  let persisted_memory_notes = store.load_memory_notes(128)?;
  let persisted_plugin_states = store.load_plugin_states()?;
  let next_thread_number = persisted_threads.len() + 1;
  let next_approval_number = store.next_approval_sequence()?;
  let next_memory_number = store.next_memory_sequence()?;

  Ok(RuntimePersistenceBootstrap {
    persistence_state: RuntimePersistenceState::persistent(store),
    memory_state: RuntimeMemoryState::new(next_memory_number, persisted_memory_notes),
    thread_state: RuntimeThreadState::new(
      persisted_threads.into_iter().map(stored_thread).collect(),
    ),
    workspace_state: RuntimeWorkspaceState::new(persisted_workspace),
    execution_state: RuntimeExecutionState::new(
      persisted_pending_approvals
        .into_iter()
        .map(|approval| (approval.id.clone(), pending_approval(approval)))
        .collect(),
      HashMap::new(),
    ),
    sequence_state: RuntimeSequenceState::new(next_thread_number, next_approval_number),
    plugin_states: persisted_plugin_states,
  })
}
