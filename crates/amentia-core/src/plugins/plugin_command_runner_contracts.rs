use std::collections::HashMap;

use serde::Deserialize;

pub(super) const PLUGIN_RUNNER_MEMORY_NOTE_LIMIT: usize = 4;
pub(super) const PLUGIN_RUNNER_MEMORY_NOTE_TITLE_LIMIT: usize = 120;
pub(super) const PLUGIN_RUNNER_MEMORY_NOTE_BODY_LIMIT: usize = 4096;
pub(super) const PLUGIN_RUNNER_MEMORY_NOTE_TAG_LIMIT: usize = 8;
pub(super) const PLUGIN_RUNNER_MEMORY_NOTE_TAG_LENGTH_LIMIT: usize = 40;
pub(super) const PLUGIN_RUNNER_OUTPUT_CONTENT_LIMIT: usize = 8192;
pub(super) const PLUGIN_RUNNER_TIMELINE_ITEM_CONTENT_LIMIT: usize = 4096;
pub(super) const PLUGIN_RUNNER_TIMELINE_ITEM_TITLE_LIMIT: usize = 120;
pub(super) const PLUGIN_RUNNER_ALLOWED_TIMELINE_KINDS: &[&str] =
  &["pluginResult", "toolResult", "warning", "system"];
pub(super) const PLUGIN_RUNNER_CONNECTOR_WORKFLOW_CONTRACT: &str = "amentia.connectorWorkflow.v1";
pub(super) const PLUGIN_RUNNER_CONNECTOR_WORKFLOW_STATUSES: &[&str] =
  &["completed", "inspected", "prepared", "retryNeeded"];
pub(super) const PLUGIN_RUNNER_REMOTE_WRITE_CONTRACT: &str = "amentia.connectorRemoteWrite.v1";
pub(super) const PLUGIN_RUNNER_REMOTE_WRITE_COMPLETED_STAGE: &str = "completed";
pub(super) const PLUGIN_RUNNER_REMOTE_WRITE_FAILED_BEFORE_PROOF_STAGE: &str = "failedBeforeProof";
pub(super) const PLUGIN_RUNNER_REMOTE_WRITE_INSPECTION_STAGE: &str = "inspectBeforeWrite";
pub(super) const PLUGIN_RUNNER_REMOTE_WRITE_STATUS_COMPLETED: &str = "completed";
pub(super) const PLUGIN_RUNNER_REMOTE_WRITE_STATUS_NOT_SENT: &str = "notSent";
pub(super) const PLUGIN_RUNNER_REMOTE_WRITE_STATUS_PENDING: &str = "pending";
pub(super) const PLUGIN_RUNNER_REMOTE_WRITE_STATUS_UNCONFIRMED: &str = "unconfirmed";

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct PluginRunnerOutputEnvelope {
  pub(super) content: Option<String>,
  pub(super) message: Option<String>,
  #[serde(default)]
  pub(super) items: Vec<PluginRunnerTimelineItemEnvelope>,
  #[serde(default)]
  pub(super) memory_notes: Vec<PluginRunnerMemoryNoteEnvelope>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct PluginRunnerTimelineItemEnvelope {
  pub(super) kind: String,
  pub(super) title: String,
  pub(super) content: String,
  #[serde(default)]
  pub(super) attributes: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct PluginRunnerMemoryNoteEnvelope {
  pub(super) title: Option<String>,
  pub(super) body: Option<String>,
  pub(super) source: Option<String>,
  #[serde(default)]
  pub(super) tags: Vec<String>,
}
