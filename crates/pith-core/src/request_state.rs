use std::collections::HashMap;

use pith_memory::{MemoryEvent, MemoryNote};
use pith_model_runtime::LocalModelRuntime;
use pith_plugin_host::PluginCatalogEntry;
use pith_protocol::{TimelineItem, WorkspaceSummary};

use crate::active_turns::ActiveTurn;
use crate::intent_inference;
use crate::plugin_hooks::PluginHookMemoryCapture;
use crate::runtime_context::PendingApproval;

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
