use anyhow::Result;
use amentia_storage::RuntimeStore;

use super::runtime_persistence_records::stored_approval_record;
use super::runtime_persistence_threads::save_threads;
use crate::approval_types::PendingApproval;
use crate::runtime_execution::RuntimeExecutionState;
use crate::runtime_threads::RuntimeThreadState;

pub(super) fn save_pending_approvals(
  store: Option<&RuntimeStore>,
  execution_state: &RuntimeExecutionState,
) -> Result<()> {
  let Some(store) = store else {
    return Ok(());
  };

  let approvals = execution_state
    .pending_approval_snapshots()
    .into_iter()
    .map(stored_approval_record)
    .collect::<Vec<_>>();

  store.save_pending_approvals(&approvals)
}

pub(super) fn save_runtime_state(
  store: Option<&RuntimeStore>,
  thread_state: &RuntimeThreadState,
  execution_state: &RuntimeExecutionState,
) -> Result<()> {
  save_threads(store, thread_state)?;
  save_pending_approvals(store, execution_state)
}

pub(super) fn resolve_approval(
  store: Option<&RuntimeStore>,
  approval: &PendingApproval,
  decision: &str,
) -> Result<()> {
  let Some(store) = store else {
    return Ok(());
  };

  store.resolve_approval(&stored_approval_record(approval.clone()), decision)
}

pub(super) fn delete_approvals_for_thread(
  store: Option<&RuntimeStore>,
  thread_id: &str,
) -> Result<usize> {
  let Some(store) = store else {
    return Ok(0);
  };

  store.delete_approvals_for_thread(thread_id)
}
