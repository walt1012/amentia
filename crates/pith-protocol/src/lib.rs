use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

pub mod methods {
  pub const APPROVAL_RESPOND: &str = "approval/respond";
  pub const INITIALIZE: &str = "initialize";
  pub const HEALTH_PING: &str = "health/ping";
  pub const MEMORY_CREATE: &str = "memory/create";
  pub const MEMORY_LIST: &str = "memory/list";
  pub const MEMORY_STATUS: &str = "memory/status";
  pub const MODEL_BOOTSTRAP: &str = "model/bootstrap";
  pub const MODEL_HEALTH: &str = "model/health";
  pub const PLUGIN_CAPABILITY_REGISTRY: &str = "plugin/capabilityRegistry";
  pub const PLUGIN_COMMAND_REGISTRY: &str = "plugin/commandRegistry";
  pub const PLUGIN_COMMAND_RUN: &str = "plugin/commandRun";
  pub const PLUGIN_HOOK_REGISTRY: &str = "plugin/hookRegistry";
  pub const PLUGIN_LIST: &str = "plugin/list";
  pub const PLUGIN_SET_ENABLED: &str = "plugin/setEnabled";
  pub const THREAD_UPDATED_NOTIFICATION: &str = "thread/updated";
  pub const WORKSPACE_CURRENT: &str = "workspace/current";
  pub const WORKSPACE_OPEN: &str = "workspace/open";
  pub const TURN_CANCEL: &str = "turn/cancel";
  pub const THREAD_READ: &str = "thread/read";
  pub const THREAD_START: &str = "thread/start";
  pub const THREAD_LIST: &str = "thread/list";
  pub const TURN_START: &str = "turn/start";
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
  pub id: Value,
  pub method: String,
  #[serde(default)]
  pub params: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
  pub id: Value,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub result: Option<Value>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub error: Option<RpcError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcNotification {
  pub method: String,
  #[serde(default)]
  pub params: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcError {
  pub code: i32,
  pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientInfo {
  pub name: String,
  pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializeParams {
  pub client_info: ClientInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerInfo {
  pub name: String,
  pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerCapabilities {
  pub supports_memory: bool,
  pub supports_threads: bool,
  pub supports_tools: bool,
  pub supports_plugins: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializeResult {
  pub server_info: ServerInfo,
  pub protocol_version: String,
  pub capabilities: ServerCapabilities,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthPingResult {
  pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelHealthResult {
  pub pack_id: String,
  pub display_name: String,
  pub backend: String,
  pub status: String,
  pub detail: String,
  pub source: String,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub binary_path: Option<String>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub model_path: Option<String>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub manifest_path: Option<String>,
  #[serde(default)]
  pub metrics: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelBootstrapResult {
  pub manifest_path: String,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub readme_path: Option<String>,
  pub copied_files: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryNoteSummary {
  pub id: String,
  pub title: String,
  pub body: String,
  pub scope: String,
  pub source: String,
  pub created_at: i64,
  pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryStatusResult {
  pub note_count: usize,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub latest_title: Option<String>,
  pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryListResult {
  pub notes: Vec<MemoryNoteSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryCreateParams {
  pub title: String,
  pub body: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryCreateResult {
  pub note: MemoryNoteSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginSummary {
  pub id: String,
  pub name: String,
  pub version: String,
  pub display_name: String,
  pub status: String,
  pub description: String,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub author_name: Option<String>,
  pub enabled: bool,
  pub default_enabled: bool,
  pub capabilities: Vec<String>,
  pub permissions: Vec<String>,
  pub manifest_path: String,
  pub provenance: String,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub validation_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginListResult {
  pub plugins: Vec<PluginSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginCapabilityRegistration {
  pub capability_id: String,
  pub kind: String,
  pub identifier: String,
  pub plugin_id: String,
  pub plugin_display_name: String,
  pub permissions: Vec<String>,
  pub manifest_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginCapabilityRegistrySummary {
  pub enabled_plugin_count: usize,
  pub total_capability_count: usize,
  pub capability_counts_by_kind: HashMap<String, usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginCapabilityRegistryResult {
  pub capabilities: Vec<PluginCapabilityRegistration>,
  pub summary: PluginCapabilityRegistrySummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginCommandSummary {
  pub command_id: String,
  pub title: String,
  pub description: String,
  pub plugin_id: String,
  pub plugin_display_name: String,
  pub permissions: Vec<String>,
  pub source_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginCommandRegistryResult {
  pub commands: Vec<PluginCommandSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginCommandRunParams {
  pub thread_id: String,
  pub command_id: String,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub input: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginHookSummary {
  pub hook_id: String,
  pub title: String,
  pub description: String,
  pub event: String,
  pub plugin_id: String,
  pub plugin_display_name: String,
  pub permissions: Vec<String>,
  pub source_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginHookRegistryResult {
  pub hooks: Vec<PluginHookSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginSetEnabledParams {
  pub plugin_id: String,
  pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginSetEnabledResult {
  pub plugin: PluginSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadSummary {
  pub id: String,
  pub title: String,
  pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceSummary {
  pub root_path: String,
  pub display_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceOpenParams {
  pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceOpenResult {
  pub workspace: WorkspaceSummary,
  pub thread_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceCurrentResult {
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub workspace: Option<WorkspaceSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadListResult {
  pub threads: Vec<ThreadSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadStartParams {
  pub title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadStartResult {
  pub thread: ThreadSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadReadParams {
  pub thread_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TurnStartParams {
  pub thread_id: String,
  pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineItem {
  pub kind: String,
  pub title: String,
  pub content: String,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub attributes: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApprovalRequest {
  pub id: String,
  pub thread_id: String,
  pub action: String,
  pub title: String,
  pub relative_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApprovalRespondParams {
  pub approval_id: String,
  pub decision: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadReadResult {
  pub thread: ThreadSummary,
  pub items: Vec<TimelineItem>,
  pub pending_approvals: Vec<ApprovalRequest>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub active_turn_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadUpdatedNotificationParams {
  pub thread: ThreadSummary,
  pub items: Vec<TimelineItem>,
  pub pending_approvals: Vec<ApprovalRequest>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub active_turn_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TurnStartResult {
  pub turn_id: String,
  pub thread_id: String,
  pub items: Vec<TimelineItem>,
  pub pending_approvals: Vec<ApprovalRequest>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub active_turn_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TurnCancelParams {
  pub turn_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TurnCancelResult {
  pub turn_id: String,
  pub thread_id: String,
  pub items: Vec<TimelineItem>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub active_turn_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApprovalRespondResult {
  pub approval_id: String,
  pub thread_id: String,
  pub items: Vec<TimelineItem>,
  pub pending_approvals: Vec<ApprovalRequest>,
}

impl JsonRpcResponse {
  pub fn success<T>(id: Value, result: &T) -> Self
  where
    T: Serialize,
  {
    Self {
      id,
      result: Some(serde_json::to_value(result).expect("serializable result")),
      error: None,
    }
  }

  pub fn error(id: Value, code: i32, message: impl Into<String>) -> Self {
    Self {
      id,
      result: None,
      error: Some(RpcError {
        code,
        message: message.into(),
      }),
    }
  }
}
