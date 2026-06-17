use std::collections::HashMap;

use amentia_memory::{MemoryEvent, MemoryNote};
use amentia_model_runtime::{GenerationCancellation, LocalModelRuntime};
use amentia_plugin_host::PluginCatalogEntry;
use amentia_protocol::{TimelineItem, WorkspaceSummary};
use amentia_storage::StoredWorkspaceChangeRecord;

use crate::active_turns::ActiveTurn;
use crate::approval_types::PendingApproval;
use crate::intent_inference;
use crate::plugin_commands::{PluginCommandOutput, PluginCommandSnapshot};
use crate::plugin_hooks::PluginHookMemoryCapture;
use crate::requests::approval_agent_context::ApprovalAgentContext;
use crate::runtime_plugins::RuntimePluginState;
use crate::turn::local_execution_safety::{LocalChangeExecutionPolicy, LocalExecutionSafetyMode};

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
  pub(crate) cancellation: GenerationCancellation,
  pub(crate) memory_notes: Vec<MemoryNote>,
  pub(crate) plugin_state: RuntimePluginState,
  pub(crate) permission_sources: HashMap<String, Vec<String>>,
  pub(crate) local_execution_safety_mode: LocalExecutionSafetyMode,
  pub(crate) reserved_approval_ids: Vec<String>,
  pub(crate) action: PreparedTurnAction,
}

#[derive(Debug)]
pub(crate) enum PreparedTurnAction {
  NoWorkspace,
  Write {
    intent: intent_inference::WriteIntent,
    policy: LocalChangeExecutionPolicy,
  },
  Shell {
    command: String,
    policy: LocalChangeExecutionPolicy,
  },
  PluginCommand {
    snapshot: Box<PluginCommandSnapshot>,
  },
  PluginCommandRouteFailed {
    command_id: String,
    message: String,
    attributes: HashMap<String, String>,
  },
  ReadFile {
    relative_path: String,
  },
  Search {
    query: String,
  },
  WebSearch(intent_inference::WebSearchIntent),
  WebSearchCandidate(intent_inference::WebSearchIntent),
  ListWorkspace,
}

#[derive(Debug)]
pub(crate) struct TurnStartExecutionOutput {
  pub(crate) thread_id: String,
  pub(crate) turn_id: String,
  pub(crate) items: Vec<TimelineItem>,
  pub(crate) pending_approval: Option<PendingApproval>,
  pub(crate) pending_active_turn: Option<ActiveTurn>,
  pub(crate) plugin_command_outputs: Vec<PluginCommandOutput>,
}

#[derive(Debug)]
pub(crate) struct PreparedApprovalSnapshot {
  pub(crate) approval: PendingApproval,
  pub(crate) decision: String,
  pub(crate) workspace: WorkspaceSummary,
  pub(crate) agent_context: ApprovalAgentContext,
  pub(crate) model_runtime: LocalModelRuntime,
  pub(crate) cancellation: GenerationCancellation,
  pub(crate) memory_notes: Vec<MemoryNote>,
  pub(crate) permission_sources: HashMap<String, Vec<String>>,
  pub(crate) plugins: Vec<PluginCatalogEntry>,
  pub(crate) approved_plugin_command: Option<PluginCommandSnapshot>,
}

#[derive(Debug)]
pub(crate) struct ApprovalExecutionOutput {
  pub(crate) approval: PendingApproval,
  pub(crate) decision: String,
  pub(crate) workspace: WorkspaceSummary,
  pub(crate) items: Vec<TimelineItem>,
  pub(crate) memory_event: Option<MemoryEvent>,
  pub(crate) hook_memory_captures: Vec<PluginHookMemoryCapture>,
  pub(crate) approved_plugin_command_output: Option<PluginCommandOutput>,
  pub(crate) workspace_changes: Vec<StoredWorkspaceChangeRecord>,
}
