use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::Result;
use amentia_plugin_host::{discover_plugins_in_roots, PluginCatalogEntry};

pub(crate) fn load_plugin_catalog(plugin_roots: &[PathBuf]) -> Result<Vec<PluginCatalogEntry>> {
  if plugin_roots.is_empty() {
    return Ok(vec![]);
  }

  discover_plugins_in_roots(plugin_roots)
}

pub(crate) fn apply_plugin_states(
  mut plugins: Vec<PluginCatalogEntry>,
  persisted_states: &HashMap<String, bool>,
) -> Vec<PluginCatalogEntry> {
  for plugin in &mut plugins {
    if plugin.status != "ready" {
      plugin.enabled = false;
      continue;
    }
    if let Some(enabled) = persisted_states.get(&plugin.id) {
      plugin.enabled = *enabled;
    }
  }

  plugins
}

#[cfg(test)]
mod tests {
  use super::*;

  fn plugin(id: &str, status: &str, enabled: bool) -> PluginCatalogEntry {
    PluginCatalogEntry {
      id: id.to_string(),
      name: id.to_string(),
      version: "1.0.0".to_string(),
      display_name: id.to_string(),
      status: status.to_string(),
      description: "Test plugin".to_string(),
      author_name: Some("Amentia".to_string()),
      enabled,
      default_enabled: enabled,
      capabilities: vec![],
      permissions: vec![],
      manifest_path: format!("/tmp/{id}/amentia-plugin.json"),
      provenance: "local".to_string(),
      validation_error: None,
      validation_hint: None,
    }
  }

  #[test]
  fn empty_plugin_roots_return_empty_catalog() {
    let catalog = load_plugin_catalog(&[]).expect("load empty catalog");

    assert!(catalog.is_empty());
  }

  #[test]
  fn persisted_states_only_apply_to_ready_plugins() {
    let plugins = vec![
      plugin("ready-enabled", "ready", false),
      plugin("ready-disabled", "ready", true),
      plugin("invalid", "invalid", true),
    ];
    let states = HashMap::from([
      ("ready-enabled".to_string(), true),
      ("ready-disabled".to_string(), false),
      ("invalid".to_string(), true),
    ]);

    let plugins = apply_plugin_states(plugins, &states);

    assert!(plugins[0].enabled);
    assert!(!plugins[1].enabled);
    assert!(!plugins[2].enabled);
  }
}
