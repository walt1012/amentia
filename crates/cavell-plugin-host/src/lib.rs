use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginAuthor {
  pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginManifest {
  pub name: String,
  pub version: String,
  pub display_name: String,
  pub description: String,
  #[serde(default)]
  pub author: Option<PluginAuthor>,
  #[serde(default)]
  pub capabilities: Vec<String>,
  #[serde(default)]
  pub permissions: Vec<String>,
  pub default_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginCatalogEntry {
  pub id: String,
  pub name: String,
  pub version: String,
  pub display_name: String,
  pub description: String,
  pub author_name: Option<String>,
  pub enabled: bool,
  pub default_enabled: bool,
  pub capabilities: Vec<String>,
  pub permissions: Vec<String>,
  pub manifest_path: String,
  pub provenance: String,
}

pub fn default_plugin_root() -> Option<PathBuf> {
  if let Ok(path) = env::var("CAVELL_PLUGIN_DIR") {
    return Some(PathBuf::from(path));
  }

  let roots = discovery_roots();
  for root in &roots {
    let candidate = root.join("plugins");
    if candidate.exists() {
      return Some(candidate);
    }
  }

  roots.into_iter().next().map(|root| root.join("plugins"))
}

pub fn discover_plugins(root: &Path) -> Result<Vec<PluginCatalogEntry>> {
  if !root.exists() {
    return Ok(vec![]);
  }

  let mut manifests = vec![];
  discover_plugin_manifests(root, &mut manifests)?;

  let mut plugins = manifests
    .into_iter()
    .map(load_plugin_entry)
    .collect::<Result<Vec<_>>>()?;

  plugins.sort_by(|left, right| left.display_name.cmp(&right.display_name));
  Ok(plugins)
}

fn discover_plugin_manifests(directory: &Path, manifests: &mut Vec<PathBuf>) -> Result<()> {
  for entry in fs::read_dir(directory)
    .with_context(|| format!("failed to read plugin directory {}", directory.display()))?
  {
    let entry =
      entry.with_context(|| format!("failed to inspect plugin entry in {}", directory.display()))?;
    let path = entry.path();

    if path.is_dir() {
      discover_plugin_manifests(&path, manifests)?;
      continue;
    }

    if path
      .file_name()
      .and_then(|name| name.to_str())
      .is_some_and(|name| name == "cavell-plugin.json")
    {
      manifests.push(path);
    }
  }

  Ok(())
}

fn load_plugin_entry(manifest_path: PathBuf) -> Result<PluginCatalogEntry> {
  let manifest = read_manifest(&manifest_path)?;
  let provenance = if manifest_path
    .components()
    .any(|component| component.as_os_str() == "official")
  {
    "official"
  } else {
    "local"
  };

  Ok(PluginCatalogEntry {
    id: manifest.name.clone(),
    name: manifest.name.clone(),
    version: manifest.version,
    display_name: manifest.display_name,
    description: manifest.description,
    author_name: manifest.author.map(|author| author.name),
    enabled: manifest.default_enabled,
    default_enabled: manifest.default_enabled,
    capabilities: manifest.capabilities,
    permissions: manifest.permissions,
    manifest_path: manifest_path.display().to_string(),
    provenance: provenance.to_string(),
  })
}

fn read_manifest(path: &Path) -> Result<PluginManifest> {
  let content =
    fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
  serde_json::from_str(&content)
    .with_context(|| format!("failed to parse plugin manifest {}", path.display()))
}

fn discovery_roots() -> Vec<PathBuf> {
  let mut roots = vec![];

  if let Ok(current_executable) = env::current_exe() {
    if let Some(parent) = current_executable.parent() {
      roots.push(parent.to_path_buf());
    }
  }

  if let Ok(current_directory) = env::current_dir() {
    roots.push(current_directory);
  }

  let mut unique_roots = vec![];
  for root in roots {
    if !unique_roots.contains(&root) {
      unique_roots.push(root);
    }
  }

  unique_roots
}
