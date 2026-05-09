use pith_plugin_host::PluginCatalogEntry;
use pith_protocol::JsonRpcRequest;
use serde_json::{json, Value};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::runtime_context::RuntimeContext;

pub(crate) fn request(method: &str, params: Option<Value>) -> JsonRpcRequest {
  JsonRpcRequest {
    id: json!(1),
    method: method.to_string(),
    params,
  }
}

pub(crate) fn create_temp_workspace(label: &str) -> PathBuf {
  let unique = SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .expect("system time")
    .as_nanos();
  let path = env::temp_dir().join(format!("pith-{label}-{unique}"));
  fs::create_dir_all(&path).expect("create temp workspace");
  path
}

pub(crate) fn create_temp_plugin_bundle(
  label: &str,
  plugin_name: &str,
  display_name: &str,
) -> PathBuf {
  let root = create_temp_workspace(label);
  let plugin_dir = root.join(plugin_name);
  fs::create_dir_all(plugin_dir.join("commands")).expect("create plugin commands directory");
  fs::write(
    plugin_dir.join("pith-plugin.json"),
    format!(
      r#"{{
"name": "{plugin_name}",
"version": "0.1.0",
"displayName": "{display_name}",
"description": "Temporary test plugin",
"author": {{ "name": "Pith" }},
"capabilities": ["command:{plugin_name}.run"],
"permissions": ["file.read"],
"defaultEnabled": true
}}"#
    ),
  )
  .expect("write plugin manifest");
  fs::write(
    plugin_dir
      .join("commands")
      .join(format!("{plugin_name}.run.json")),
    r#"{
"title": "Run Temporary Plugin",
"description": "Execute a temporary plugin command.",
"prompt": "Summarize the local workspace in one paragraph."
}"#,
  )
  .expect("write command manifest");
  plugin_dir
}

pub(crate) fn replace_plugin_catalog(
  context: &mut RuntimeContext,
  catalog: Vec<PluginCatalogEntry>,
) {
  context.plugin_state.replace_catalog(catalog);
}

pub(crate) fn bundled_plugin_entry(
  id: &str,
  display_name: &str,
  enabled: bool,
  default_enabled: bool,
  capabilities: &[&str],
  permissions: &[&str],
) -> PluginCatalogEntry {
  PluginCatalogEntry {
    id: id.to_string(),
    name: id.to_string(),
    version: "0.1.0".to_string(),
    display_name: display_name.to_string(),
    status: "ready".to_string(),
    description: "Test plugin".to_string(),
    author_name: Some("Pith".to_string()),
    enabled,
    default_enabled,
    capabilities: capabilities
      .iter()
      .map(|capability| capability.to_string())
      .collect(),
    permissions: permissions
      .iter()
      .map(|permission| permission.to_string())
      .collect(),
    manifest_path: format!("plugins/bundled/{id}/pith-plugin.json"),
    provenance: "bundled".to_string(),
    validation_error: None,
    validation_hint: None,
  }
}

pub(crate) fn enable_full_access_plugin(context: &mut RuntimeContext) {
  replace_plugin_catalog(
    context,
    vec![PluginCatalogEntry {
      id: "test-full-access".to_string(),
      name: "test-full-access".to_string(),
      version: "0.1.0".to_string(),
      display_name: "Test Full Access".to_string(),
      status: "ready".to_string(),
      description: "Grants built-in workspace and shell permissions for tests".to_string(),
      author_name: Some("Pith".to_string()),
      enabled: true,
      default_enabled: true,
      capabilities: vec!["prompt_pack:test.full_access".to_string()],
      permissions: vec![
        "file.read".to_string(),
        "file.write".to_string(),
        "shell.exec".to_string(),
      ],
      manifest_path: "tests/test-full-access/pith-plugin.json".to_string(),
      provenance: "test".to_string(),
      validation_error: None,
      validation_hint: None,
    }],
  );
}
