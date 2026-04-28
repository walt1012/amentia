use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

use crate::manifest::{PluginCommandManifest, PluginHookManifest, PluginManifest};

pub(crate) fn read_manifest(path: &Path) -> Result<PluginManifest> {
  let content =
    fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
  serde_json::from_str(&content)
    .with_context(|| format!("failed to parse plugin manifest {}", path.display()))
}

pub(crate) fn read_command_manifest(path: &Path) -> Result<PluginCommandManifest> {
  let content =
    fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
  serde_json::from_str(&content)
    .with_context(|| format!("failed to parse plugin command {}", path.display()))
}

pub(crate) fn read_hook_manifest(path: &Path) -> Result<PluginHookManifest> {
  let content =
    fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
  serde_json::from_str(&content)
    .with_context(|| format!("failed to parse plugin hook {}", path.display()))
}
