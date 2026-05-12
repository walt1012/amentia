use pith_protocol::{ThreadSummary, TimelineItem, WorkspaceSummary};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoragePaths {
  pub database_path: String,
  pub artifacts_path: String,
  pub plugins_path: String,
  pub runtime_state_path: String,
}

impl StoragePaths {
  pub fn application_support_defaults() -> Self {
    Self {
      database_path: "~/Library/Application Support/Pith/storage/pith.db".to_string(),
      artifacts_path: "~/Library/Application Support/Pith/artifacts".to_string(),
      plugins_path: "~/Library/Application Support/Pith/plugins".to_string(),
      runtime_state_path: "~/Library/Application Support/Pith/storage/threads.json".to_string(),
    }
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredThreadRecord {
  pub summary: ThreadSummary,
  pub turn_count: usize,
  pub items: Vec<TimelineItem>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub workspace: Option<WorkspaceSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredApprovalRecord {
  pub id: String,
  pub thread_id: String,
  pub action: String,
  pub title: String,
  pub relative_path: String,
  pub content: Option<String>,
  pub command: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StoredPluginConnectorCredential {
  pub connector_id: String,
  pub plugin_id: String,
  pub credential_store: String,
  pub credential_label: String,
  #[serde(default)]
  pub credential_secret: Option<String>,
  pub authorized_at: i64,
  pub updated_at: i64,
}
