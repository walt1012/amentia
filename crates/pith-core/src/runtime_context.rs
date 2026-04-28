use std::collections::HashMap;
use std::path::PathBuf;

use pith_memory::{MemoryManager, MemoryNote};
use pith_model_runtime::LocalModelRuntime;
use pith_plugin_host::PluginCatalogEntry;
use pith_protocol::WorkspaceSummary;
use pith_storage::FileThreadStore;

use crate::active_turns::ActiveTurn;
use crate::approval_types::PendingApproval;
use crate::runtime_sequences::RuntimeSequenceState;
use crate::thread_state::StoredThread;

#[derive(Debug, Clone)]
pub struct RuntimeContext {
  pub(crate) server_name: String,
  pub(crate) server_version: String,
  pub(crate) model_runtime: LocalModelRuntime,
  pub(crate) memory_manager: MemoryManager,
  pub(crate) store: Option<FileThreadStore>,
  pub(crate) memory_notes: Vec<MemoryNote>,
  pub(crate) threads: Vec<StoredThread>,
  pub(crate) workspace: Option<WorkspaceSummary>,
  pub(crate) plugin_roots: Vec<PathBuf>,
  pub(crate) plugin_install_root: PathBuf,
  pub(crate) plugins: Vec<PluginCatalogEntry>,
  pub(crate) pending_approvals: HashMap<String, PendingApproval>,
  pub(crate) active_turns: HashMap<String, ActiveTurn>,
  pub(crate) enforce_model_readiness: bool,
  pub(crate) sequences: RuntimeSequenceState,
}
