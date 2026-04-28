use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::io::read_manifest;
use crate::types::{PluginCatalogEntry, PluginRemovalRecord};
use crate::validation::validate_manifest;

pub fn install_plugin_bundle(
  source_path: &Path,
  install_root: &Path,
) -> Result<PluginCatalogEntry> {
  let source_root = resolve_plugin_source_root(source_path)?;
  let plugin = inspect_plugin_bundle(&source_root)?;
  let manifest_name = plugin.name.clone();

  fs::create_dir_all(install_root).with_context(|| {
    format!(
      "failed to create plugin install root {}",
      install_root.display()
    )
  })?;

  let destination_root = install_root.join(&manifest_name);
  if destination_root.exists() {
    anyhow::bail!(
      "plugin `{}` is already installed at {}",
      plugin.display_name,
      destination_root.display()
    );
  }

  copy_directory(&source_root, &destination_root)?;
  Ok(crate::catalog::load_plugin_entry(destination_root.join("pith-plugin.json")))
}

pub fn inspect_plugin_bundle(source_path: &Path) -> Result<PluginCatalogEntry> {
  let source_root = resolve_plugin_source_root(source_path)?;
  let source_manifest_path = source_root.join("pith-plugin.json");
  let manifest = read_manifest(&source_manifest_path)?;
  validate_manifest(&manifest)?;
  Ok(crate::catalog::load_plugin_entry(source_manifest_path))
}

pub fn remove_local_plugin_bundle(
  manifest_path: &Path,
  install_root: &Path,
) -> Result<PluginRemovalRecord> {
  let resolved_manifest_path = fs::canonicalize(manifest_path).with_context(|| {
    format!(
      "failed to resolve plugin manifest {}",
      manifest_path.display()
    )
  })?;
  let plugin_entry = crate::catalog::load_plugin_entry(resolved_manifest_path.clone());
  let plugin_root = resolved_manifest_path
    .parent()
    .context("plugin manifest is missing a parent directory")?
    .to_path_buf();
  let resolved_install_root = canonicalize_or_prepare(install_root)?;

  if !plugin_root.starts_with(&resolved_install_root) {
    anyhow::bail!(
      "plugin removal is only supported for locally installed plugins under {}",
      resolved_install_root.display()
    );
  }
  if plugin_entry.provenance != "local" {
    anyhow::bail!("plugin removal is only supported for local plugins");
  }

  fs::remove_dir_all(&plugin_root)
    .with_context(|| format!("failed to remove plugin bundle {}", plugin_root.display()))?;

  Ok(PluginRemovalRecord {
    plugin_id: plugin_entry.id,
    display_name: plugin_entry.display_name,
    removed_path: plugin_root.display().to_string(),
  })
}

fn resolve_plugin_source_root(path: &Path) -> Result<PathBuf> {
  let source_path = fs::canonicalize(path)
    .with_context(|| format!("failed to resolve plugin source {}", path.display()))?;

  if source_path.is_dir() {
    let manifest_path = source_path.join("pith-plugin.json");
    if !manifest_path.is_file() {
      anyhow::bail!(
        "plugin directory {} does not contain pith-plugin.json",
        source_path.display()
      );
    }
    return Ok(source_path);
  }

  if source_path
    .file_name()
    .and_then(|name| name.to_str())
    .is_some_and(|name| name == "pith-plugin.json")
  {
    return source_path
      .parent()
      .map(Path::to_path_buf)
      .context("plugin manifest is missing a parent directory");
  }

  anyhow::bail!("plugin source must be a plugin directory or pith-plugin.json file");
}

fn copy_directory(source: &Path, destination: &Path) -> Result<()> {
  fs::create_dir_all(destination)
    .with_context(|| format!("failed to create {}", destination.display()))?;

  for entry in fs::read_dir(source)
    .with_context(|| format!("failed to read plugin directory {}", source.display()))?
  {
    let entry = entry.with_context(|| format!("failed to inspect {}", source.display()))?;
    let entry_path = entry.path();
    let destination_path = destination.join(entry.file_name());

    if entry_path.is_dir() {
      copy_directory(&entry_path, &destination_path)?;
    } else {
      fs::copy(&entry_path, &destination_path).with_context(|| {
        format!(
          "failed to copy {} to {}",
          entry_path.display(),
          destination_path.display()
        )
      })?;
    }
  }

  Ok(())
}

fn canonicalize_or_prepare(path: &Path) -> Result<PathBuf> {
  if path.exists() {
    fs::canonicalize(path)
      .with_context(|| format!("failed to resolve plugin install root {}", path.display()))
  } else {
    fs::create_dir_all(path)
      .with_context(|| format!("failed to create plugin install root {}", path.display()))?;
    fs::canonicalize(path)
      .with_context(|| format!("failed to resolve plugin install root {}", path.display()))
  }
}
