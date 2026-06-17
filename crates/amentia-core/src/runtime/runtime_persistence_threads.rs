use amentia_storage::RuntimeStore;
use anyhow::Result;

use super::runtime_persistence_records::stored_thread_record;
use crate::runtime_threads::RuntimeThreadState;

pub(super) fn save_threads(
  store: Option<&RuntimeStore>,
  thread_state: &RuntimeThreadState,
) -> Result<()> {
  let Some(store) = store else {
    return Ok(());
  };

  let threads = thread_state
    .iter()
    .map(stored_thread_record)
    .collect::<Vec<_>>();

  store.save_threads(&threads)
}
