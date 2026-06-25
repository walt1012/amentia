use amentia_plugin_host::PluginCatalogEntry;
use amentia_protocol::JsonRpcRequest;
use serde_json::{json, Value};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
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
  let path = env::temp_dir().join(format!("amentia-{label}-{unique}"));
  fs::create_dir_all(&path).expect("create temp workspace");
  path
}

pub(crate) fn remove_temp_workspace(workspace: &Path) {
  fs::remove_dir_all(workspace).expect("cleanup temp workspace");
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
    plugin_dir.join("amentia-plugin.json"),
    format!(
      r#"{{
"name": "{plugin_name}",
"version": "0.1.0",
"displayName": "{display_name}",
"description": "Temporary test plugin",
"author": {{ "name": "Amentia" }},
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

pub(crate) fn remove_temp_plugin_source(source_root: &Path) {
  let root = source_root.parent().expect("plugin root");
  fs::remove_dir_all(root).expect("cleanup plugin source");
}

pub(crate) fn replace_plugin_catalog(
  context: &mut RuntimeContext,
  catalog: Vec<PluginCatalogEntry>,
) {
  context.plugin_state.replace_catalog(catalog);
}

fn string_list(items: &[&str]) -> Vec<String> {
  items.iter().map(|item| item.to_string()).collect()
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
    author_name: Some("Amentia".to_string()),
    enabled,
    default_enabled,
    capabilities: string_list(capabilities),
    permissions: string_list(permissions),
    manifest_path: format!("plugins/bundled/{id}/amentia-plugin.json"),
    provenance: "bundled".to_string(),
    validation_error: None,
    validation_hint: None,
  }
}

pub(crate) fn bundled_manifest_plugin_entry(
  id: &str,
  display_name: &str,
  enabled: bool,
  default_enabled: bool,
  capabilities: &[&str],
  permissions: &[&str],
) -> PluginCatalogEntry {
  let mut plugin = bundled_plugin_entry(
    id,
    display_name,
    enabled,
    default_enabled,
    capabilities,
    permissions,
  );
  plugin.manifest_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    .join(format!("../../plugins/bundled/{id}/amentia-plugin.json"))
    .display()
    .to_string();
  plugin
}

pub(crate) fn temp_manifest_plugin_entry(
  id: &str,
  display_name: &str,
  description: &str,
  capabilities: &[&str],
  permissions: &[&str],
  manifest_path: &Path,
) -> PluginCatalogEntry {
  PluginCatalogEntry {
    id: id.to_string(),
    name: id.to_string(),
    version: "0.1.0".to_string(),
    display_name: display_name.to_string(),
    status: "ready".to_string(),
    description: description.to_string(),
    author_name: Some("Amentia".to_string()),
    enabled: true,
    default_enabled: true,
    capabilities: string_list(capabilities),
    permissions: string_list(permissions),
    manifest_path: manifest_path.display().to_string(),
    provenance: "test".to_string(),
    validation_error: None,
    validation_hint: None,
  }
}

#[cfg(unix)]
pub(crate) fn make_test_file_executable(path: &Path, label: &str) {
  use std::os::unix::fs::PermissionsExt;

  let mut permissions = fs::metadata(path)
    .unwrap_or_else(|error| panic!("{label} metadata: {error}"))
    .permissions();
  permissions.set_mode(0o755);
  fs::set_permissions(path, permissions)
    .unwrap_or_else(|error| panic!("make {label} executable: {error}"));
}

#[cfg(not(unix))]
pub(crate) fn make_test_file_executable(_path: &Path, _label: &str) {}

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
      author_name: Some("Amentia".to_string()),
      enabled: true,
      default_enabled: true,
      capabilities: vec!["skill:test.full_access".to_string()],
      permissions: vec![
        "file.read".to_string(),
        "file.write".to_string(),
        "shell.exec".to_string(),
      ],
      manifest_path: "tests/test-full-access/amentia-plugin.json".to_string(),
      provenance: "test".to_string(),
      validation_error: None,
      validation_hint: None,
    }],
  );
}
