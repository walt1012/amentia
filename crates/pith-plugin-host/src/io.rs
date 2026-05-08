use std::fs::File;
use std::io::Read;
use std::path::Path;

use anyhow::{bail, Context, Result};
use serde::de::DeserializeOwned;

use crate::manifest::{PluginCommandManifest, PluginHookManifest, PluginManifest};

const PLUGIN_MANIFEST_MAX_BYTES: usize = 64 * 1024;

pub(crate) fn read_manifest(path: &Path) -> Result<PluginManifest> {
  read_json_manifest(path, "plugin manifest")
}

pub(crate) fn read_command_manifest(path: &Path) -> Result<PluginCommandManifest> {
  read_json_manifest(path, "plugin command")
}

pub(crate) fn read_hook_manifest(path: &Path) -> Result<PluginHookManifest> {
  read_json_manifest(path, "plugin hook")
}

fn read_json_manifest<T>(path: &Path, label: &str) -> Result<T>
where
  T: DeserializeOwned,
{
  let content = read_bounded_manifest_content(path, label)?;
  serde_json::from_str(&content)
    .with_context(|| format!("failed to parse {label} {}", path.display()))
}

fn read_bounded_manifest_content(path: &Path, label: &str) -> Result<String> {
  let mut file =
    File::open(path).with_context(|| format!("failed to read {label} {}", path.display()))?;
  let read_limit = PLUGIN_MANIFEST_MAX_BYTES.saturating_add(1);
  let mut bytes = Vec::with_capacity(read_limit.min(16 * 1024));
  let mut buffer = [0_u8; 4096];

  while bytes.len() < read_limit {
    let remaining = read_limit.saturating_sub(bytes.len());
    let read_size = remaining.min(buffer.len());
    let bytes_read = file
      .read(&mut buffer[..read_size])
      .with_context(|| format!("failed to read {label} {}", path.display()))?;
    if bytes_read == 0 {
      break;
    }
    bytes.extend_from_slice(&buffer[..bytes_read]);
  }

  if bytes.len() > PLUGIN_MANIFEST_MAX_BYTES {
    bail!(
      "{} {} exceeds the {} byte limit",
      label,
      path.display(),
      PLUGIN_MANIFEST_MAX_BYTES
    );
  }

  String::from_utf8(bytes).with_context(|| format!("{label} {} is not valid UTF-8", path.display()))
}

#[cfg(test)]
mod tests {
  use std::fs;
  use std::path::PathBuf;
  use std::time::{SystemTime, UNIX_EPOCH};

  use super::*;

  #[test]
  fn read_manifest_rejects_oversized_json() {
    let root = unique_temp_directory("oversized-manifest");
    fs::create_dir_all(&root).expect("root");
    let manifest_path = root.join("pith-plugin.json");
    fs::write(&manifest_path, "x".repeat(PLUGIN_MANIFEST_MAX_BYTES + 1)).expect("manifest");

    let error = read_manifest(&manifest_path).expect_err("oversized manifest should fail");

    assert!(error.to_string().contains("exceeds"));

    let _ = fs::remove_dir_all(root);
  }

  fn unique_temp_directory(prefix: &str) -> PathBuf {
    let nonce = SystemTime::now()
      .duration_since(UNIX_EPOCH)
      .expect("clock")
      .as_nanos();
    std::env::temp_dir().join(format!("pith-plugin-host-{prefix}-{nonce}"))
  }
}
