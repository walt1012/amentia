use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub validation_hint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginListResult {
  pub plugins: Vec<PluginSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginRefreshResult {
  pub plugins: Vec<PluginSummary>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub state_warning: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginInstallParams {
  pub source_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginInspectParams {
  pub source_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginInspectResult {
  pub plugin: PluginSummary,
  pub install_status: String,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub install_blocker: Option<String>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub install_repair_hint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginInstallResult {
  pub plugin: PluginSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginRemoveParams {
  pub manifest_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginRemoveResult {
  pub plugin_id: String,
  pub display_name: String,
  pub removed_path: String,
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
  #[serde(default, skip_serializing_if = "HashMap::is_empty")]
  pub metadata: HashMap<String, String>,
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
pub struct PluginConnectorSummary {
  pub connector_id: String,
  pub display_name: String,
  pub service: String,
  pub plugin_id: String,
  pub plugin_display_name: String,
  pub enabled: bool,
  pub status: String,
  pub permissions: Vec<String>,
  pub manifest_path: String,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub homepage: Option<String>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub auth_type: Option<String>,
  pub auth_required: bool,
  #[serde(default)]
  pub auth_scopes: Vec<String>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub credential_store: Option<String>,
  #[serde(default, skip_serializing_if = "Vec::is_empty")]
  pub workflows: Vec<PluginConnectorWorkflowSummary>,
  pub auth_status: String,
  pub credential_present: bool,
  pub credential_secret_present: bool,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub credential_provider: Option<String>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub credential_handle: Option<String>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub credential_label: Option<String>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub authorized_at: Option<i64>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub credential_updated_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginConnectorWorkflowSummary {
  pub workflow_id: String,
  pub display_name: String,
  pub connector_id: String,
  pub service: String,
  pub action: String,
  #[serde(default)]
  pub stages: Vec<String>,
  #[serde(default)]
  pub statuses: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginConnectorRegistryResult {
  pub connectors: Vec<PluginConnectorSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginConnectorCredentialParams {
  pub connector_id: String,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub credential_label: Option<String>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub credential_secret: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginConnectorCredentialResult {
  pub connector: PluginConnectorSummary,
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
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub execution: Option<PluginCommandExecutionSummary>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub execution_kind: Option<String>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub memory_summary: Option<String>,
  pub run_status: String,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub run_blocker: Option<String>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub run_repair_hint: Option<String>,
  #[serde(default)]
  pub declared_connector_ids: Vec<String>,
  #[serde(default)]
  pub required_connector_ids: Vec<String>,
  #[serde(default)]
  pub approval_required: bool,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub approval_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginCommandExecutionSummary {
  pub kind: String,
  pub driver: String,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub entrypoint: Option<String>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub workflow_id: Option<String>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub workflow: Option<PluginCommandWorkflowSummary>,
  pub input: PluginCommandEnvelopeSummary,
  pub output: PluginCommandEnvelopeSummary,
  pub supported: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginCommandWorkflowSummary {
  pub workflow_id: String,
  pub display_name: String,
  pub connector_id: String,
  pub service: String,
  pub action: String,
  #[serde(default)]
  pub stages: Vec<String>,
  #[serde(default)]
  pub statuses: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginCommandEnvelopeSummary {
  pub envelope: String,
  #[serde(default)]
  pub fields: Vec<PluginCommandEnvelopeFieldSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginCommandEnvelopeFieldSummary {
  pub name: String,
  pub kind: String,
  pub required: bool,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub description: Option<String>,
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
  pub status: String,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub run_blocker: Option<String>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub run_repair_hint: Option<String>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub memory_summary: Option<String>,
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
