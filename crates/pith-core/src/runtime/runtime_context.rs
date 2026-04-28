use std::collections::HashMap;

use pith_memory::{MemoryManager, MemoryNote};
use pith_model_runtime::LocalModelRuntime;
use pith_protocol::WorkspaceSummary;
use pith_storage::RuntimeStore;

use crate::active_turns::ActiveTurn;
use crate::approval_types::PendingApproval;
use crate::runtime_identity::RuntimeIdentity;
use crate::runtime_plugins::RuntimePluginState;
use crate::runtime_sequences::RuntimeSequenceState;
use crate::thread_state::StoredThread;

#[derive(Debug, Clone)]
pub struct RuntimeContext {
  pub(crate) identity: RuntimeIdentity,
  pub(crate) model_runtime: LocalModelRuntime,
  pub(crate) memory_manager: MemoryManager,
  pub(crate) store: Option<RuntimeStore>,
  pub(crate) memory_notes: Vec<MemoryNote>,
  pub(crate) threads: Vec<StoredThread>,
  pub(crate) workspace: Option<WorkspaceSummary>,
  pub(crate) plugin_state: RuntimePluginState,
  pub(crate) pending_approvals: HashMap<String, PendingApproval>,
  pub(crate) active_turns: HashMap<String, ActiveTurn>,
  pub(crate) enforce_model_readiness: bool,
  pub(crate) sequences: RuntimeSequenceState,
}
