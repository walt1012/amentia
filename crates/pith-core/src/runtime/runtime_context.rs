use pith_storage::RuntimeStore;

use crate::runtime_execution::RuntimeExecutionState;
use crate::runtime_identity::RuntimeIdentity;
use crate::runtime_memory::RuntimeMemoryState;
use crate::runtime_model::RuntimeModelState;
use crate::runtime_plugins::RuntimePluginState;
use crate::runtime_sequences::RuntimeSequenceState;
use crate::runtime_workspace::RuntimeWorkspaceState;
use crate::thread_state::StoredThread;

#[derive(Debug, Clone)]
pub struct RuntimeContext {
  pub(crate) identity: RuntimeIdentity,
  pub(crate) model_state: RuntimeModelState,
  pub(crate) store: Option<RuntimeStore>,
  pub(crate) memory_state: RuntimeMemoryState,
  pub(crate) threads: Vec<StoredThread>,
  pub(crate) workspace_state: RuntimeWorkspaceState,
  pub(crate) plugin_state: RuntimePluginState,
  pub(crate) execution_state: RuntimeExecutionState,
  pub(crate) sequences: RuntimeSequenceState,
}
