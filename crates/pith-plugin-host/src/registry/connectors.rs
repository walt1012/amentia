use std::path::Path;

use crate::io::read_manifest;
use crate::types::{PluginCatalogEntry, PluginConnectorEntry, PluginConnectorWorkflowEntry};

use super::workflow_commands::connector_workflow_command_ids;

pub fn build_connector_registry(plugins: &[PluginCatalogEntry]) -> Vec<PluginConnectorEntry> {
  let mut connectors = vec![];

  for plugin in plugins.iter().filter(|plugin| plugin.status == "ready") {
    let Ok(manifest) = read_manifest(Path::new(&plugin.manifest_path)) else {
      continue;
    };
    let Some(plugin_root) = Path::new(&plugin.manifest_path).parent() else {
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

    for connector in &manifest.app_connectors {
      let workflows = manifest
        .connector_workflows
        .iter()
        .filter(|workflow| workflow.connector_id.as_str() == connector.id.as_str())
        .map(|workflow| PluginConnectorWorkflowEntry {
          workflow_id: workflow.id.clone(),
          display_name: workflow.display_name.clone(),
          connector_id: workflow.connector_id.clone(),
          service: connector.service.clone(),
          action: workflow.action.clone(),
          max_agent_steps: workflow.max_agent_steps,
          stages: workflow.stages.clone(),
          statuses: workflow.statuses.clone(),
          command_ids: connector_workflow_command_ids(plugin, plugin_root, &workflow.id),
        })
        .collect();
      connectors.push(PluginConnectorEntry {
        connector_id: format!("{}::{}", plugin.id, connector.id),
        display_name: connector.display_name.clone(),
        service: connector.service.clone(),
        plugin_id: plugin.id.clone(),
        plugin_display_name: plugin.display_name.clone(),
        enabled: plugin.enabled,
        status: status.to_string(),
        permissions: plugin.permissions.clone(),
        manifest_path: plugin.manifest_path.clone(),
        homepage: connector.homepage.clone(),
        auth_type: auth_type.clone(),
        auth_required,
        auth_scopes: auth_scopes.clone(),
        credential_store: credential_store.clone(),
        workflows,
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
