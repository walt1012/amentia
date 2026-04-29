use std::collections::HashMap;

use anyhow::Result;
use pith_memory::MemoryNote;
use pith_protocol::WorkspaceSummary;
use pith_storage::{RuntimeStore, StoredApprovalRecord, StoredThreadRecord};

use crate::approval_types::PendingApproval;
use crate::runtime_execution::RuntimeExecutionState;
use crate::runtime_memory::RuntimeMemoryState;
use crate::runtime_sequences::RuntimeSequenceState;
use crate::runtime_threads::RuntimeThreadState;
use crate::runtime_workspace::RuntimeWorkspaceState;
use crate::thread_state::StoredThread;

pub(crate) struct RuntimePersistenceBootstrap {
  pub(crate) persistence_state: RuntimePersistenceState,
  pub(crate) memory_state: RuntimeMemoryState,
  pub(crate) thread_state: RuntimeThreadState,
  pub(crate) workspace_state: RuntimeWorkspaceState,
  pub(crate) execution_state: RuntimeExecutionState,
  pub(crate) sequence_state: RuntimeSequenceState,
  pub(crate) plugin_states: HashMap<String, bool>,
}

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
        persisted_threads
          .into_iter()
          .map(stored_thread_record)
          .collect(),
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

  pub(crate) fn save_runtime_state(
    &self,
    thread_state: &RuntimeThreadState,
    execution_state: &RuntimeExecutionState,
  ) -> Result<()> {
    self.save_threads(thread_state)?;
    self.save_pending_approvals(execution_state)
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

fn stored_thread_record(thread: StoredThreadRecord) -> StoredThread {
  StoredThread::new(
    thread.summary,
    thread.turn_count,
    thread.items,
    thread.workspace,
  )
}

fn pending_approval(approval: StoredApprovalRecord) -> PendingApproval {
  PendingApproval {
    id: approval.id,
    thread_id: approval.thread_id,
    action: approval.action,
    title: approval.title,
    relative_path: approval.relative_path,
    content: approval.content,
    command: approval.command,
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  use std::env;
  use std::fs;
  use std::time::{SystemTime, UNIX_EPOCH};

  use pith_protocol::{ThreadSummary, TimelineItem};

  fn create_temp_directory(label: &str) -> std::path::PathBuf {
    let unique = SystemTime::now()
      .duration_since(UNIX_EPOCH)
      .expect("system time")
      .as_nanos();
    let path = env::temp_dir().join(format!("pith-core-persistence-{label}-{unique}"));
    fs::create_dir_all(&path).expect("create temp directory");
    path
  }

  #[test]
  fn bootstrap_maps_storage_records_into_runtime_state() {
    let root = create_temp_directory("bootstrap");
    let store = RuntimeStore::new(root.join("pith.db"), root.join("threads.json"));
    let workspace = WorkspaceSummary {
      root_path: "/tmp/pith".to_string(),
      display_name: "pith".to_string(),
    };
    let thread = StoredThreadRecord {
      summary: ThreadSummary {
        id: "thread-1".to_string(),
        title: "Thread".to_string(),
        status: "ready".to_string(),
        workspace: Some(workspace.clone()),
      },
      turn_count: 2,
      items: vec![TimelineItem {
        kind: "system".to_string(),
        title: "Thread Ready".to_string(),
        content: "Ready".to_string(),
        attributes: None,
      }],
      workspace: Some(workspace.clone()),
    };
    let approval = StoredApprovalRecord {
      id: "approval-3".to_string(),
      thread_id: "thread-1".to_string(),
      action: "write_file".to_string(),
      title: "Write docs/output.txt".to_string(),
      relative_path: "docs/output.txt".to_string(),
      content: Some("hello".to_string()),
      command: None,
    };
    let note = MemoryNote {
      id: "memory-4".to_string(),
      title: "Opened workspace pith".to_string(),
      body: "Pith opened the workspace at /tmp/pith.".to_string(),
      scope: "pith".to_string(),
      source: "workspace".to_string(),
      created_at: 4,
      tags: vec!["workspace".to_string()],
    };

    store.save_workspace(&workspace).expect("save workspace");
    store.save_threads(&[thread]).expect("save thread records");
    store
      .save_pending_approvals(&[approval])
      .expect("save pending approval records");
    store.save_memory_note(&note).expect("save memory note");
    store
      .save_plugin_enabled("notion", true)
      .expect("save plugin state");

    let mut bootstrap = RuntimePersistenceState::load_bootstrap(store).expect("load bootstrap");

    assert_eq!(bootstrap.thread_state.count_for_workspace(&workspace), 1);
    assert_eq!(
      bootstrap
        .workspace_state
        .current()
        .map(|workspace| workspace.display_name.as_str()),
      Some("pith")
    );
    assert_eq!(
      bootstrap
        .execution_state
        .approval_requests_for_thread("thread-1")
        .len(),
      1
    );
    assert_eq!(bootstrap.memory_state.note_count(), 1);
    assert_eq!(bootstrap.plugin_states.get("notion"), Some(&true));
    assert_eq!(bootstrap.sequence_state.next_approval_id(), "approval-4");

    fs::remove_dir_all(root).expect("cleanup temp directory");
  }
}
