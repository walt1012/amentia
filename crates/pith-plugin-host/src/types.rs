use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginCatalogEntry {
  pub id: String,
  pub name: String,
  pub version: String,
  pub display_name: String,
  pub status: String,
  pub description: String,
  pub author_name: Option<String>,
  pub enabled: bool,
  pub default_enabled: bool,
  pub capabilities: Vec<String>,
  pub permissions: Vec<String>,
  pub manifest_path: String,
  pub provenance: String,
  pub validation_error: Option<String>,
  pub validation_hint: Option<String>,
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
  pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginConnectorEntry {
  pub connector_id: String,
  pub display_name: String,
  pub service: String,
  pub plugin_id: String,
  pub plugin_display_name: String,
  pub enabled: bool,
  pub status: String,
  pub permissions: Vec<String>,
  pub manifest_path: String,
  pub homepage: Option<String>,
  pub auth_type: Option<String>,
  pub auth_required: bool,
  pub auth_scopes: Vec<String>,
  pub credential_store: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginCommandEntry {
  pub command_id: String,
  pub title: String,
  pub description: String,
  pub prompt: String,
  pub plugin_id: String,
  pub plugin_display_name: String,
  pub permissions: Vec<String>,
  pub source_path: String,
  pub execution: Option<PluginCommandExecutionEntry>,
  pub execution_kind: Option<String>,
  pub manifest_error: Option<String>,
  pub memory_note_title: Option<String>,
  pub memory_note_source: Option<String>,
  pub memory_note_tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginCommandExecutionEntry {
  pub kind: String,
  pub driver: String,
  pub entrypoint: Option<String>,
  pub connector_ids: Option<Vec<String>>,
  pub workflow_id: Option<String>,
  pub input: PluginCommandEnvelopeEntry,
  pub output: PluginCommandEnvelopeEntry,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginCommandEnvelopeEntry {
  pub envelope: String,
  pub fields: Vec<PluginCommandEnvelopeFieldEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginCommandEnvelopeFieldEntry {
  pub name: String,
  pub kind: String,
  pub required: bool,
  pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginHookEntry {
  pub hook_id: String,
  pub title: String,
  pub description: String,
  pub event: String,
  pub message_template: String,
  pub plugin_id: String,
  pub plugin_display_name: String,
  pub permissions: Vec<String>,
  pub source_path: String,
  pub manifest_error: Option<String>,
  pub memory_note_title: Option<String>,
  pub memory_note_source: Option<String>,
  pub memory_note_tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginRemovalRecord {
  pub plugin_id: String,
  pub display_name: String,
  pub removed_path: String,
}
