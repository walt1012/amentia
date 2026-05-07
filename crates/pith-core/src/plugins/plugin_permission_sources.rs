use std::collections::HashMap;

use pith_plugin_host::PluginCatalogEntry;

pub(crate) fn granted_permission_sources(
  plugins: &[PluginCatalogEntry],
) -> HashMap<String, Vec<String>> {
  let mut permissions = HashMap::new();

  for plugin in plugins
    .iter()
    .filter(|plugin| plugin.status == "ready" && plugin.enabled)
  {
    for permission in &plugin.permissions {
      permissions
        .entry(permission.clone())
        .or_insert_with(Vec::new)
        .push(plugin.display_name.clone());
    }
  }

  for plugin_names in permissions.values_mut() {
    plugin_names.sort();
    plugin_names.dedup();
  }

  permissions
}

pub(crate) fn permission_is_granted(
  permission_sources: &HashMap<String, Vec<String>>,
  permission: &str,
) -> bool {
  permission_sources.contains_key(permission)
}

#[cfg(test)]
mod tests {
  use super::*;

  fn plugin(id: &str, name: &str, status: &str, enabled: bool) -> PluginCatalogEntry {
    PluginCatalogEntry {
      id: id.to_string(),
      name: id.to_string(),
      version: "1.0.0".to_string(),
      display_name: name.to_string(),
      status: status.to_string(),
      description: "Test plugin".to_string(),
      author_name: Some("Pith".to_string()),
      enabled,
      default_enabled: enabled,
      capabilities: vec![],
      permissions: vec!["file.read".to_string(), "file.read".to_string()],
      manifest_path: format!("/tmp/{id}/pith-plugin.json"),
      provenance: "local".to_string(),
      validation_error: None,
      validation_hint: None,
    }
  }

  #[test]
  fn granted_permission_sources_only_include_ready_enabled_plugins() {
    let sources = granted_permission_sources(&[
      plugin("enabled", "Enabled Plugin", "ready", true),
      plugin("disabled", "Disabled Plugin", "ready", false),
      plugin("invalid", "Invalid Plugin", "invalid", true),
    ]);

    let expected = vec!["Enabled Plugin".to_string()];
    assert_eq!(sources.get("file.read"), Some(&expected));
    assert!(permission_is_granted(&sources, "file.read"));
    assert!(!permission_is_granted(&sources, "shell.exec"));
  }
}
