use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
pub struct ModelProbeResult {
  pub status: String,
  pub detail: String,
  pub backend: String,
  pub model_id: String,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub sample: Option<String>,
}
