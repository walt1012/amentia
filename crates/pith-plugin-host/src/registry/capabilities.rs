use crate::types::{PluginCapabilityRegistration, PluginCatalogEntry};

use super::metadata::plugin_capability_metadata;

pub fn build_capability_registry(
  plugins: &[PluginCatalogEntry],
) -> Vec<PluginCapabilityRegistration> {
  let mut registrations = plugins
    .iter()
    .filter(|plugin| plugin.status == "ready" && plugin.enabled)
    .flat_map(|plugin| {
      let metadata_by_capability = plugin_capability_metadata(plugin);
      plugin
        .capabilities
        .iter()
        .filter_map(|capability| {
          let (kind, identifier) = capability.split_once(':')?;
          Some(PluginCapabilityRegistration {
            capability_id: format!("{}::{}", plugin.id, capability),
            kind: kind.to_string(),
            identifier: identifier.to_string(),
            plugin_id: plugin.id.clone(),
            plugin_display_name: plugin.display_name.clone(),
            permissions: plugin.permissions.clone(),
            manifest_path: plugin.manifest_path.clone(),
            metadata: metadata_by_capability
              .get(capability)
              .cloned()
              .unwrap_or_default(),
          })
        })
        .collect::<Vec<_>>()
    })
    .collect::<Vec<_>>();

  registrations.sort_by(|left, right| {
    left
      .kind
      .cmp(&right.kind)
      .then_with(|| left.identifier.cmp(&right.identifier))
      .then_with(|| left.plugin_id.cmp(&right.plugin_id))
  });
  registrations
}
