use std::collections::HashMap;

use pith_plugin_host::PluginCatalogEntry;
use pith_protocol::TimelineItem;

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

pub(crate) fn build_permission_denied_items(
  permission_sources: &HashMap<String, Vec<String>>,
  permission: &str,
  blocked_action: &str,
  workspace_name: &str,
  mut attributes: HashMap<String, String>,
) -> Vec<TimelineItem> {
  let granted_by = permission_sources
    .get(permission)
    .map(|plugins| plugins.join(", "))
    .unwrap_or_else(|| "none".to_string());
  attributes.insert("requiredPermission".to_string(), permission.to_string());
  attributes.insert("blockedAction".to_string(), blocked_action.to_string());
  attributes.insert("grantedBy".to_string(), granted_by.clone());

  vec![
    TimelineItem {
      kind: "warning".to_string(),
      title: "Plugin Permission Required".to_string(),
      content: format!(
        "Pith could not {blocked_action} in {workspace_name} because no enabled plugin grants `{permission}`."
      ),
      attributes: Some(attributes.clone()),
    },
    TimelineItem {
      kind: "assistantMessage".to_string(),
      title: "Assistant".to_string(),
      content: format!(
        "Enable a plugin that grants `{permission}` before asking Pith to {blocked_action}. Currently granted by: {granted_by}."
      ),
      attributes: Some(attributes),
    },
  ]
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

  #[test]
  fn permission_denied_items_include_required_permission_metadata() {
    let items = build_permission_denied_items(
      &HashMap::new(),
      "shell.exec",
      "run a shell command",
      "pith",
      HashMap::from([("turnId".to_string(), "turn-1".to_string())]),
    );
    let attributes = items[0].attributes.as_ref().expect("attributes");

    assert_eq!(
      attributes.get("requiredPermission"),
      Some(&"shell.exec".to_string())
    );
    assert_eq!(attributes.get("grantedBy"), Some(&"none".to_string()));
  }
}
