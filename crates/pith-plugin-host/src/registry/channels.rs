use std::path::Path;

use crate::io::read_manifest;
use crate::types::{PluginCatalogEntry, PluginChannelEntry};

pub fn build_channel_registry(plugins: &[PluginCatalogEntry]) -> Vec<PluginChannelEntry> {
  let mut channels = vec![];

  for plugin in plugins.iter().filter(|plugin| plugin.status == "ready") {
    let Ok(manifest) = read_manifest(Path::new(&plugin.manifest_path)) else {
      continue;
    };
    let status = if plugin.enabled { "ready" } else { "disabled" };
    for channel in manifest.app_channels {
      channels.push(PluginChannelEntry {
        channel_id: format!("{}::{}", plugin.id, channel.id),
        display_name: channel.display_name,
        service: channel.service,
        protocol: channel.protocol,
        plugin_id: plugin.id.clone(),
        plugin_display_name: plugin.display_name.clone(),
        enabled: plugin.enabled,
        status: status.to_string(),
        permissions: plugin.permissions.clone(),
        manifest_path: plugin.manifest_path.clone(),
        homepage: channel.homepage,
      });
    }
  }

  channels.sort_by(|left, right| {
    left
      .service
      .cmp(&right.service)
      .then_with(|| left.display_name.cmp(&right.display_name))
      .then_with(|| left.channel_id.cmp(&right.channel_id))
  });
  channels
}
