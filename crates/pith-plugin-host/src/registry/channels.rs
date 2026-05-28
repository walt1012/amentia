use std::path::Path;

use crate::io::read_manifest;
use crate::types::{PluginCatalogEntry, PluginChannelEntry};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginChannelAdapterBlocker {
  pub display_name: String,
  pub service: String,
  pub protocol: String,
  pub message: String,
}

pub fn channel_adapter_blocker_for_manifest(
  manifest_path: &Path,
) -> anyhow::Result<Option<PluginChannelAdapterBlocker>> {
  let manifest = read_manifest(manifest_path)?;

  Ok(
    manifest
      .app_channels
      .into_iter()
      .filter_map(|channel| {
        let readiness = channel_adapter_readiness(&channel.protocol);
        (!readiness.available).then(|| PluginChannelAdapterBlocker {
          display_name: channel.display_name,
          service: channel.service,
          protocol: channel.protocol,
          message: readiness.blocker,
        })
      })
      .next(),
  )
}

pub fn build_channel_registry(plugins: &[PluginCatalogEntry]) -> Vec<PluginChannelEntry> {
  let mut channels = vec![];

  for plugin in plugins.iter().filter(|plugin| plugin.status == "ready") {
    let Ok(manifest) = read_manifest(Path::new(&plugin.manifest_path)) else {
      continue;
    };
    for channel in manifest.app_channels {
      let readiness = channel_adapter_readiness(&channel.protocol);
      let status = if !plugin.enabled {
        "disabled"
      } else if readiness.available {
        "ready"
      } else {
        "adapterPending"
      };
      let activation_blocker = (!readiness.available).then(|| readiness.blocker.clone());
      channels.push(PluginChannelEntry {
        channel_id: format!("{}::{}", plugin.id, channel.id),
        display_name: channel.display_name,
        service: channel.service,
        protocol: channel.protocol,
        supports_inbound: channel.supports_inbound,
        supports_outbound: channel.supports_outbound,
        approval_required: channel.approval_required,
        safety_notes: channel.safety_notes,
        adapter_status: readiness.status.to_string(),
        adapter_available: readiness.available,
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

#[derive(Debug, Clone)]
struct ChannelAdapterReadiness {
  status: &'static str,
  available: bool,
  blocker: String,
}

fn channel_adapter_readiness(protocol: &str) -> ChannelAdapterReadiness {
  match protocol {
    "openclaw-weixin" => ChannelAdapterReadiness {
      status: "feasibilityPending",
      available: false,
      blocker: "Weixin channel is paused at feasibility. The official OpenClaw Weixin package is an OpenClaw host plugin; prove QR login, getUpdates, sendMessage, and recovery without a required OpenClaw runtime before enabling this channel."
        .to_string(),
    },
    _ => ChannelAdapterReadiness {
      status: "pending",
      available: false,
      blocker: format!("Channel adapter for protocol `{protocol}` is not available yet."),
    },
  }
}
