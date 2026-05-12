use std::path::Path;

use crate::io::read_manifest;
use crate::types::{PluginCatalogEntry, PluginConnectorEntry};

pub fn build_connector_registry(plugins: &[PluginCatalogEntry]) -> Vec<PluginConnectorEntry> {
  let mut connectors = vec![];

  for plugin in plugins.iter().filter(|plugin| plugin.status == "ready") {
    let Ok(manifest) = read_manifest(Path::new(&plugin.manifest_path)) else {
      continue;
    };

    let auth_type = manifest
      .auth_policy
      .as_ref()
      .map(|policy| policy.auth_type.clone());
    let auth_required = manifest
      .auth_policy
      .as_ref()
      .map(|policy| policy.required)
      .unwrap_or(false);
    let auth_scopes = manifest
      .auth_policy
      .as_ref()
      .map(|policy| policy.scopes.clone())
      .unwrap_or_default();
    let credential_store = manifest
      .auth_policy
      .as_ref()
      .and_then(|policy| policy.credential_store.clone());
    let status = if !plugin.enabled {
      "disabled"
    } else if auth_required {
      "needsAuth"
    } else {
      "ready"
    };

    for connector in manifest.app_connectors {
      connectors.push(PluginConnectorEntry {
        connector_id: format!("{}::{}", plugin.id, connector.id),
        display_name: connector.display_name,
        service: connector.service,
        plugin_id: plugin.id.clone(),
        plugin_display_name: plugin.display_name.clone(),
        enabled: plugin.enabled,
        status: status.to_string(),
        permissions: plugin.permissions.clone(),
        manifest_path: plugin.manifest_path.clone(),
        homepage: connector.homepage,
        auth_type: auth_type.clone(),
        auth_required,
        auth_scopes: auth_scopes.clone(),
        credential_store: credential_store.clone(),
      });
    }
  }

  connectors.sort_by(|left, right| {
    left
      .service
      .cmp(&right.service)
      .then_with(|| left.display_name.cmp(&right.display_name))
      .then_with(|| left.connector_id.cmp(&right.connector_id))
  });
  connectors
}
