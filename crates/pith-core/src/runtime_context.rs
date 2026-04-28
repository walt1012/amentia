use std::collections::HashMap;
use std::path::PathBuf;

use pith_memory::{MemoryEvent, MemoryManager, MemoryNote};
use pith_model_runtime::LocalModelRuntime;
use pith_plugin_host::PluginCatalogEntry;
use pith_protocol::{ThreadSummary, TimelineItem, WorkspaceSummary};
use pith_storage::FileThreadStore;

use crate::active_turns::ActiveTurn;
use crate::intent_inference;
use crate::plugin_hooks::PluginHookMemoryCapture;

#[derive(Debug, Clone)]
pub(crate) struct StoredThread {
  pub(crate) summary: ThreadSummary,
  pub(crate) turn_count: usize,
  pub(crate) items: Vec<TimelineItem>,
  pub(crate) workspace: Option<WorkspaceSummary>,
}

#[derive(Debug, Clone)]
pub(crate) struct PendingApproval {
  pub(crate) id: String,
  pub(crate) thread_id: String,
  pub(crate) action: String,
  pub(crate) title: String,
  pub(crate) relative_path: String,
  pub(crate) content: Option<String>,
  pub(crate) command: Option<String>,
}

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
  pub(crate) next_thread_number: usize,
  pub(crate) next_approval_number: usize,
}

#[derive(Debug)]
pub struct PreparedTurnStart {
  pub(crate) request_id: serde_json::Value,
  pub(crate) snapshot: PreparedTurnSnapshot,
}

#[derive(Debug)]
pub struct CompletedTurnStart {
  pub(crate) request_id: serde_json::Value,
  pub(crate) output: TurnStartExecutionOutput,
}

#[derive(Debug)]
pub struct PreparedApprovalRespond {
  pub(crate) request_id: serde_json::Value,
  pub(crate) snapshot: PreparedApprovalSnapshot,
}

#[derive(Debug)]
pub struct CompletedApprovalRespond {
  pub(crate) request_id: serde_json::Value,
  pub(crate) output: ApprovalExecutionOutput,
}

#[derive(Debug)]
pub(crate) struct PreparedTurnSnapshot {
  pub(crate) thread_id: String,
  pub(crate) turn_id: String,
  pub(crate) thread_title: String,
  pub(crate) display_message: String,
  pub(crate) message: String,
  pub(crate) workspace: Option<WorkspaceSummary>,
  pub(crate) model_runtime: LocalModelRuntime,
  pub(crate) memory_notes: Vec<MemoryNote>,
  pub(crate) permission_sources: HashMap<String, Vec<String>>,
  pub(crate) action: PreparedTurnAction,
}

#[derive(Debug)]
pub(crate) enum PreparedTurnAction {
  NoWorkspace,
  Write {
    intent: intent_inference::WriteIntent,
    approval_id: Option<String>,
  },
  Shell {
    command: String,
    approval_id: Option<String>,
  },
  ReadFile {
    relative_path: String,
  },
  Search {
    query: String,
  },
  ListWorkspace,
}

#[derive(Debug)]
pub(crate) struct TurnStartExecutionOutput {
  pub(crate) thread_id: String,
  pub(crate) turn_id: String,
  pub(crate) items: Vec<TimelineItem>,
  pub(crate) pending_approval: Option<PendingApproval>,
  pub(crate) pending_active_turn: Option<ActiveTurn>,
}

#[derive(Debug)]
pub(crate) struct PreparedApprovalSnapshot {
  pub(crate) approval: PendingApproval,
  pub(crate) decision: String,
  pub(crate) workspace: WorkspaceSummary,
  pub(crate) model_runtime: LocalModelRuntime,
  pub(crate) memory_notes: Vec<MemoryNote>,
  pub(crate) permission_sources: HashMap<String, Vec<String>>,
  pub(crate) plugins: Vec<PluginCatalogEntry>,
}

#[derive(Debug)]
pub(crate) struct ApprovalExecutionOutput {
  pub(crate) approval: PendingApproval,
  pub(crate) decision: String,
  pub(crate) workspace: WorkspaceSummary,
  pub(crate) items: Vec<TimelineItem>,
  pub(crate) memory_event: Option<MemoryEvent>,
  pub(crate) hook_memory_captures: Vec<PluginHookMemoryCapture>,
}
