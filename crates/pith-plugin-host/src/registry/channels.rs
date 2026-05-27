use std::path::Path;

use crate::io::read_manifest;
use crate::types::{PluginCatalogEntry, PluginChannelEntry};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginChannelAdapterBlocker {
  pub display_name: String,
  pub service: String,
  pub protocol: String,
}

pub fn channel_adapter_blocker_for_manifest(
  manifest_path: &Path,
) -> anyhow::Result<Option<PluginChannelAdapterBlocker>> {
  let manifest = read_manifest(manifest_path)?;

  Ok(
    manifest
      .app_channels
      .into_iter()
      .find(|channel| !channel_adapter_available(&channel.protocol))
      .map(|channel| PluginChannelAdapterBlocker {
        display_name: channel.display_name,
        service: channel.service,
        protocol: channel.protocol,
      }),
  )
}

pub fn build_channel_registry(plugins: &[PluginCatalogEntry]) -> Vec<PluginChannelEntry> {
  let mut channels = vec![];

  for plugin in plugins.iter().filter(|plugin| plugin.status == "ready") {
    let Ok(manifest) = read_manifest(Path::new(&plugin.manifest_path)) else {
      continue;
    };
    for channel in manifest.app_channels {
      let adapter_available = channel_adapter_available(&channel.protocol);
      let adapter_status = if adapter_available {
        "ready"
      } else {
        "pending"
      };
      let status = if !plugin.enabled {
        "disabled"
      } else if adapter_available {
        "ready"
      } else {
        "adapterPending"
      };
      let activation_blocker = (!adapter_available).then(|| {
        format!(
          "Channel adapter for protocol `{}` is not available yet.",
          channel.protocol
        )
      });
      channels.push(PluginChannelEntry {
        channel_id: format!("{}::{}", plugin.id, channel.id),
        display_name: channel.display_name,
        service: channel.service,
        protocol: channel.protocol,
        adapter_status: adapter_status.to_string(),
        adapter_available,
        activation_blocker,
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

fn channel_adapter_available(_protocol: &str) -> bool {
  false
}
