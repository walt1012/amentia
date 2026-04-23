use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
  pub name: String,
  pub version: String,
  pub display_name: String,
  pub description: String,
  pub default_enabled: bool,
}
