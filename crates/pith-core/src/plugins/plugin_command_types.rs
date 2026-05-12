use pith_memory::MemoryNote;
use pith_model_runtime::GenerationCancellation;
use pith_plugin_host::PluginCommandEntry as HostPluginCommandEntry;
use pith_protocol::{TimelineItem, WorkspaceSummary};
use serde::Serialize;

#[derive(Debug)]
pub struct PreparedPluginCommandRun {
  pub(super) request_id: serde_json::Value,
  pub(super) snapshot: PluginCommandSnapshot,
}

#[derive(Debug)]
pub struct CompletedPluginCommandRun {
  pub(super) request_id: serde_json::Value,
  pub(super) running_id: String,
  pub(super) output: std::result::Result<PluginCommandOutput, (i32, String)>,
}

#[derive(Debug)]
pub(super) struct PluginCommandSnapshot {
  pub(super) thread_id: String,
  pub(super) command: HostPluginCommandEntry,
  pub(super) workspace: Option<WorkspaceSummary>,
  pub(super) input: Option<String>,
  pub(super) connector_refs: Vec<PluginConnectorExecutionRef>,
  pub(super) command_item: TimelineItem,
  pub(super) memory_notes: Vec<MemoryNote>,
  pub(super) cancellation: GenerationCancellation,
  pub(super) running_id: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct PluginConnectorExecutionRef {
  pub(super) connector_id: String,
  pub(super) service: String,
  pub(super) credential_provider: PluginConnectorCredentialProviderRef,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct PluginConnectorCredentialProviderRef {
  pub(super) provider: String,
  pub(super) handle: String,
  pub(super) store: String,
  pub(super) label: String,
  pub(super) authorized_at: i64,
}

#[derive(Debug)]
pub(super) struct PluginCommandOutput {
  pub(super) thread_id: String,
  pub(super) command: HostPluginCommandEntry,
  pub(super) workspace: Option<WorkspaceSummary>,
  pub(super) input: Option<String>,
  pub(super) items: Vec<TimelineItem>,
  pub(super) capture_memory: bool,
}
