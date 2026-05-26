use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginAuthor {
  pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginManifest {
  pub name: String,
  pub version: String,
  pub display_name: String,
  pub description: String,
  #[serde(default)]
  pub author: Option<PluginAuthor>,
  #[serde(default)]
  pub capabilities: Vec<String>,
  #[serde(default)]
  pub permissions: Vec<String>,
  #[serde(default)]
  pub skills: Vec<PluginSkillManifest>,
  #[serde(default)]
  pub mcp_servers: Vec<PluginMcpServerManifest>,
  #[serde(default)]
  pub app_connectors: Vec<PluginAppConnectorManifest>,
  #[serde(default)]
  pub connector_workflows: Vec<PluginConnectorWorkflowManifest>,
  #[serde(default)]
  pub auth_policy: Option<PluginAuthPolicyManifest>,
  pub default_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginSkillManifest {
  pub id: String,
  pub description: String,
  pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginMcpServerManifest {
  pub id: String,
  #[serde(default)]
  pub command: Option<String>,
  #[serde(default)]
  pub args: Vec<String>,
  #[serde(default)]
  pub transport: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginAppConnectorManifest {
  pub id: String,
  pub display_name: String,
  pub service: String,
  #[serde(default)]
  pub homepage: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginConnectorWorkflowManifest {
  pub id: String,
  pub display_name: String,
  pub connector_id: String,
  pub action: String,
  #[serde(default)]
  pub stages: Vec<String>,
  #[serde(default)]
  pub statuses: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginAuthPolicyManifest {
  #[serde(rename = "type")]
  pub auth_type: String,
  #[serde(default)]
  pub required: bool,
  #[serde(default)]
  pub scopes: Vec<String>,
  #[serde(default)]
  pub credential_store: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PluginCommandManifest {
  pub(crate) title: String,
  pub(crate) description: String,
  pub(crate) prompt: String,
  #[serde(default)]
  pub(crate) execution: Option<PluginCommandExecutionManifest>,
  #[serde(default)]
  pub(crate) memory: Option<PluginMemoryManifest>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PluginCommandExecutionManifest {
  pub(crate) kind: String,
  #[serde(default)]
  pub(crate) driver: Option<String>,
  #[serde(default)]
  pub(crate) entrypoint: Option<String>,
  #[serde(default)]
  pub(crate) connectors: Option<Vec<String>>,
  #[serde(default)]
  pub(crate) workflow_id: Option<String>,
  #[serde(default)]
  pub(crate) input: Option<PluginCommandEnvelopeManifest>,
  #[serde(default)]
  pub(crate) output: Option<PluginCommandEnvelopeManifest>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PluginCommandEnvelopeManifest {
  pub(crate) envelope: String,
  #[serde(default)]
  pub(crate) fields: Vec<PluginCommandEnvelopeFieldManifest>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PluginCommandEnvelopeFieldManifest {
  pub(crate) name: String,
  pub(crate) kind: String,
  #[serde(default)]
  pub(crate) required: bool,
  #[serde(default)]
  pub(crate) description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PluginMemoryManifest {
  pub(crate) note_title: String,
  #[serde(default)]
  pub(crate) note_source: Option<String>,
  #[serde(default)]
  pub(crate) note_tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PluginHookManifest {
  pub(crate) title: String,
  pub(crate) description: String,
  pub(crate) event: String,
  pub(crate) message_template: String,
  #[serde(default)]
  pub(crate) memory: Option<PluginMemoryManifest>,
}
