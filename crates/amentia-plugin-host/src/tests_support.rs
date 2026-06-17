use super::*;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

pub(crate) fn create_temp_plugin_root(label: &str) -> PathBuf {
  let unique = SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .expect("system time")
    .as_nanos();
  let path = env::temp_dir().join(format!("amentia-plugin-host-{label}-{unique}"));
  fs::create_dir_all(&path).expect("create temp plugin root");
  path
}

pub(crate) fn manifest(capabilities: Vec<&str>, permissions: Vec<&str>) -> PluginManifest {
  PluginManifest {
    name: "workspace-notes".to_string(),
    version: "0.1.0".to_string(),
    display_name: "Workspace Notes".to_string(),
    description: "Test plugin".to_string(),
    author: Some(PluginAuthor {
      name: "Amentia".to_string(),
    }),
    capabilities: capabilities.into_iter().map(str::to_string).collect(),
    permissions: permissions.into_iter().map(str::to_string).collect(),
    skills: vec![],
    mcp_servers: vec![],
    app_connectors: vec![],
    connector_workflows: vec![],
    auth_policy: None,
    default_enabled: true,
  }
}
