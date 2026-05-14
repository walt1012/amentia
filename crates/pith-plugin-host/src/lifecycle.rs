use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::io::read_manifest;
use crate::types::{PluginCatalogEntry, PluginRemovalRecord};
use crate::validation::{validate_manifest, validation_hint_for_error};

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

  if let Err(error) = copy_directory(&source_root, &destination_root) {
    let _ = fs::remove_dir_all(&destination_root);
    return Err(error);
  }
  Ok(crate::catalog::load_plugin_entry(
    destination_root.join("pith-plugin.json"),
  ))
}

pub fn inspect_plugin_bundle(source_path: &Path) -> Result<PluginCatalogEntry> {
  let source_root = resolve_plugin_source_root(source_path)?;
  ensure_single_root_manifest(&source_root)?;
  let source_manifest_path = source_root.join("pith-plugin.json");
  let manifest = read_manifest(&source_manifest_path)?;
  if let Err(error) = validate_manifest(&manifest) {
    let message = error.to_string();
    let hint = validation_hint_for_error(&message);
    anyhow::bail!("{message}\nHint: {hint}");
  }
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
  if plugin_root.parent() != Some(resolved_install_root.as_path()) {
    anyhow::bail!(
      "plugin removal is only supported for plugin directories directly under {}",
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

fn ensure_single_root_manifest(source_root: &Path) -> Result<()> {
  let root_manifest = source_root.join("pith-plugin.json");
  reject_nested_plugin_manifests(source_root, &root_manifest)
}

fn reject_nested_plugin_manifests(directory: &Path, root_manifest: &Path) -> Result<()> {
  for entry in fs::read_dir(directory)
    .with_context(|| format!("failed to read plugin directory {}", directory.display()))?
  {
    let entry = entry.with_context(|| format!("failed to inspect {}", directory.display()))?;
    let entry_path = entry.path();
    let metadata = fs::symlink_metadata(&entry_path)
      .with_context(|| format!("failed to inspect plugin path {}", entry_path.display()))?;
    let file_type = metadata.file_type();

    if file_type.is_symlink() {
      continue;
    }

    if metadata.is_dir() {
      reject_nested_plugin_manifests(&entry_path, root_manifest)?;
      continue;
    }

    if metadata.is_file()
      && entry_path != root_manifest
      && entry_path
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name == "pith-plugin.json")
    {
      anyhow::bail!(
        "plugin bundles cannot contain nested pith-plugin.json manifests: {}",
        entry_path.display()
      );
    }
  }

  Ok(())
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
    let metadata = fs::symlink_metadata(&entry_path)
      .with_context(|| format!("failed to inspect plugin path {}", entry_path.display()))?;
    let file_type = metadata.file_type();

    if file_type.is_symlink() {
      anyhow::bail!(
        "plugin bundles cannot contain symbolic links: {}",
        entry_path.display()
      );
    }

    if metadata.is_dir() {
      copy_directory(&entry_path, &destination_path)?;
    } else if metadata.is_file() {
      fs::copy(&entry_path, &destination_path).with_context(|| {
        format!(
          "failed to copy {} to {}",
          entry_path.display(),
          destination_path.display()
        )
      })?;
    } else {
      anyhow::bail!(
        "plugin bundles cannot contain unsupported file type: {}",
        entry_path.display()
      );
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
