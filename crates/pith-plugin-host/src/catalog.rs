use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::io::read_manifest;
use crate::types::PluginCatalogEntry;
use crate::validation::{manifest_capabilities, validate_manifest, validation_hint_for_error};

pub fn discover_plugins(root: &Path) -> Result<Vec<PluginCatalogEntry>> {
  if !root.exists() {
    return Ok(vec![]);
  }

  let mut manifests = vec![];
  discover_plugin_manifests(root, &mut manifests)?;

  let mut plugins = manifests
    .into_iter()
    .map(load_plugin_entry)
    .collect::<Vec<_>>();

  plugins.sort_by(|left, right| left.display_name.cmp(&right.display_name));
  Ok(plugins)
}

pub fn discover_plugins_in_roots(roots: &[PathBuf]) -> Result<Vec<PluginCatalogEntry>> {
  let mut plugins = vec![];

  for root in roots {
    plugins.extend(discover_plugins(root)?);
  }

  plugins.sort_by(|left, right| {
    left
      .display_name
      .cmp(&right.display_name)
      .then_with(|| left.id.cmp(&right.id))
      .then_with(|| left.manifest_path.cmp(&right.manifest_path))
  });
  plugins.dedup_by(|left, right| left.manifest_path == right.manifest_path);
  Ok(plugins)
}

pub(crate) fn load_plugin_entry(manifest_path: PathBuf) -> PluginCatalogEntry {
  let provenance = if manifest_path.components().any(|component| {
    let name = component.as_os_str();
    name == "bundled" || name == "official"
  }) {
    "bundled"
  } else {
    "local"
  };

  match read_manifest(&manifest_path) {
    Ok(manifest) => match validate_manifest(&manifest) {
      Ok(()) => {
        let capabilities = manifest_capabilities(&manifest);
        PluginCatalogEntry {
          id: manifest.name.clone(),
          name: manifest.name.clone(),
          version: manifest.version,
          display_name: manifest.display_name,
          status: "ready".to_string(),
          description: manifest.description,
          author_name: manifest.author.map(|author| author.name),
          enabled: manifest.default_enabled,
          default_enabled: manifest.default_enabled,
          capabilities,
          permissions: manifest.permissions,
          manifest_path: manifest_path.display().to_string(),
          provenance: provenance.to_string(),
          validation_error: None,
          validation_hint: None,
        }
      }
      Err(error) => invalid_plugin_entry(
        manifest_path,
        provenance,
        Some(manifest.display_name),
        error.to_string(),
      ),
    },
    Err(error) => invalid_plugin_entry(manifest_path, provenance, None, error.to_string()),
  }
}

fn discover_plugin_manifests(directory: &Path, manifests: &mut Vec<PathBuf>) -> Result<()> {
  for entry in fs::read_dir(directory)
    .with_context(|| format!("failed to read plugin directory {}", directory.display()))?
  {
    let entry = entry
      .with_context(|| format!("failed to inspect plugin entry in {}", directory.display()))?;
    let path = entry.path();
    let metadata = fs::symlink_metadata(&path)
      .with_context(|| format!("failed to inspect plugin path {}", path.display()))?;

    if metadata.file_type().is_symlink() {
      continue;
    }

    if metadata.is_dir() {
      discover_plugin_manifests(&path, manifests)?;
      continue;
    }

    if metadata.is_file()
      && path
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name == "pith-plugin.json")
    {
      manifests.push(path);
    }
  }

  Ok(())
}

fn invalid_plugin_entry(
  manifest_path: PathBuf,
  provenance: &str,
  display_name: Option<String>,
  validation_error: String,
) -> PluginCatalogEntry {
  let validation_hint = validation_hint_for_error(&validation_error);
  let fallback_name = display_name.unwrap_or_else(|| {
    manifest_path
      .parent()
      .and_then(|path| path.file_name())
      .and_then(|name| name.to_str())
      .unwrap_or("invalid-plugin")
      .to_string()
  });
  PluginCatalogEntry {
    id: format!("invalid:{}", manifest_path.display()),
    name: fallback_name.clone(),
    version: "invalid".to_string(),
    display_name: fallback_name,
    status: "invalid".to_string(),
    description: "Plugin manifest could not be loaded.".to_string(),
    author_name: None,
    enabled: false,
    default_enabled: false,
    capabilities: vec![],
    permissions: vec![],
    manifest_path: manifest_path.display().to_string(),
    provenance: provenance.to_string(),
    validation_error: Some(validation_error),
    validation_hint: Some(validation_hint),
  }
}
