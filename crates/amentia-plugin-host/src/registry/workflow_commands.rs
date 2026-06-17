use std::path::Path;

use crate::io::read_command_manifest;
use crate::types::PluginCatalogEntry;

use super::capability_identifier_is_safe;

pub(super) fn connector_workflow_command_ids(
  plugin: &PluginCatalogEntry,
  plugin_root: &Path,
  workflow_id: &str,
) -> Vec<String> {
  let mut command_ids = plugin
    .capabilities
    .iter()
    .filter_map(|capability| capability.strip_prefix("command:"))
    .filter(|identifier| capability_identifier_is_safe(identifier))
    .filter(|identifier| command_binds_workflow(plugin_root, identifier, workflow_id))
    .map(|identifier| format!("{}::{}", plugin.id, identifier))
    .collect::<Vec<_>>();
  command_ids.sort();
  command_ids
}

fn command_binds_workflow(plugin_root: &Path, identifier: &str, workflow_id: &str) -> bool {
  let command_path = plugin_root
    .join("commands")
    .join(format!("{identifier}.json"));
  read_command_manifest(&command_path)
    .ok()
    .and_then(|command| command.execution)
    .and_then(|execution| execution.workflow_id)
    .map(|command_workflow_id| command_workflow_id == workflow_id)
    .unwrap_or(false)
}
