use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

const KNOWN_CAPABILITY_KINDS: [&str; 7] = [
  "command",
  "agent",
  "prompt_pack",
  "hook",
  "tool",
  "mcp_server",
  "settings",
];
const KNOWN_PERMISSIONS: [&str; 7] = [
  "file.read",
  "file.write",
  "shell.exec",
  "network.outbound",
  "workspace.background",
  "model.invoke",
  "mcp.connect",
];

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
  pub status: String,
  pub description: String,
  pub author_name: Option<String>,
  pub enabled: bool,
  pub default_enabled: bool,
  pub capabilities: Vec<String>,
  pub permissions: Vec<String>,
  pub manifest_path: String,
  pub provenance: String,
  pub validation_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginCapabilityRegistration {
  pub capability_id: String,
  pub kind: String,
  pub identifier: String,
  pub plugin_id: String,
  pub plugin_display_name: String,
  pub permissions: Vec<String>,
  pub manifest_path: String,
}

pub fn default_plugin_root() -> Option<PathBuf> {
  if let Ok(path) = env::var("PITH_PLUGIN_DIR") {
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
    .collect::<Vec<_>>();

  plugins.sort_by(|left, right| left.display_name.cmp(&right.display_name));
  Ok(plugins)
}

fn discover_plugin_manifests(directory: &Path, manifests: &mut Vec<PathBuf>) -> Result<()> {
  for entry in fs::read_dir(directory)
    .with_context(|| format!("failed to read plugin directory {}", directory.display()))?
  {
    let entry = entry
      .with_context(|| format!("failed to inspect plugin entry in {}", directory.display()))?;
    let path = entry.path();

    if path.is_dir() {
      discover_plugin_manifests(&path, manifests)?;
      continue;
    }

    if path
      .file_name()
      .and_then(|name| name.to_str())
      .is_some_and(|name| name == "pith-plugin.json")
    {
      manifests.push(path);
    }
  }

  Ok(())
}

fn load_plugin_entry(manifest_path: PathBuf) -> PluginCatalogEntry {
  let provenance = if manifest_path
    .components()
    .any(|component| component.as_os_str() == "official")
  {
    "official"
  } else {
    "local"
  };

  match read_manifest(&manifest_path) {
    Ok(manifest) => match validate_manifest(&manifest) {
      Ok(()) => PluginCatalogEntry {
        id: manifest.name.clone(),
        name: manifest.name.clone(),
        version: manifest.version,
        display_name: manifest.display_name,
        status: "ready".to_string(),
        description: manifest.description,
        author_name: manifest.author.map(|author| author.name),
        enabled: manifest.default_enabled,
        default_enabled: manifest.default_enabled,
        capabilities: manifest.capabilities,
        permissions: manifest.permissions,
        manifest_path: manifest_path.display().to_string(),
        provenance: provenance.to_string(),
        validation_error: None,
      },
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

fn invalid_plugin_entry(
  manifest_path: PathBuf,
  provenance: &str,
  display_name: Option<String>,
  validation_error: String,
) -> PluginCatalogEntry {
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
  }
}

fn validate_manifest(manifest: &PluginManifest) -> Result<()> {
  for capability in &manifest.capabilities {
    let Some((kind, identifier)) = capability.split_once(':') else {
      anyhow::bail!(
        "plugin capability `{}` must use the `<kind>:<identifier>` format",
        capability
      );
    };
    if !KNOWN_CAPABILITY_KINDS.contains(&kind) {
      anyhow::bail!("plugin capability kind `{}` is not supported", kind);
    }
    if identifier.trim().is_empty() {
      anyhow::bail!(
        "plugin capability `{}` must include a non-empty identifier",
        capability
      );
    }
  }

  for permission in &manifest.permissions {
    if !KNOWN_PERMISSIONS.contains(&permission.as_str()) {
      anyhow::bail!("plugin permission `{}` is not supported", permission);
    }
  }

  Ok(())
}

pub fn build_capability_registry(
  plugins: &[PluginCatalogEntry],
) -> Vec<PluginCapabilityRegistration> {
  let mut registrations = plugins
    .iter()
    .filter(|plugin| plugin.status == "ready" && plugin.enabled)
    .flat_map(|plugin| {
      plugin.capabilities.iter().filter_map(|capability| {
        let (kind, identifier) = capability.split_once(':')?;
        Some(PluginCapabilityRegistration {
          capability_id: format!("{}::{}", plugin.id, capability),
          kind: kind.to_string(),
          identifier: identifier.to_string(),
          plugin_id: plugin.id.clone(),
          plugin_display_name: plugin.display_name.clone(),
          permissions: plugin.permissions.clone(),
          manifest_path: plugin.manifest_path.clone(),
        })
      })
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

#[cfg(test)]
mod tests {
  use super::*;

  fn manifest(capabilities: Vec<&str>, permissions: Vec<&str>) -> PluginManifest {
    PluginManifest {
      name: "workspace-notes".to_string(),
      version: "0.1.0".to_string(),
      display_name: "Workspace Notes".to_string(),
      description: "Test plugin".to_string(),
      author: Some(PluginAuthor {
        name: "Pith".to_string(),
      }),
      capabilities: capabilities.into_iter().map(str::to_string).collect(),
      permissions: permissions.into_iter().map(str::to_string).collect(),
      default_enabled: true,
    }
  }

  #[test]
  fn validate_manifest_accepts_typed_capabilities_and_permissions() {
    let manifest = manifest(
      vec![
        "prompt_pack:workspace.notes",
        "settings:workspace.preferences",
      ],
      vec!["file.read", "file.write"],
    );

    let result = validate_manifest(&manifest);

    assert!(result.is_ok());
  }

  #[test]
  fn validate_manifest_rejects_legacy_capability_format() {
    let manifest = manifest(vec!["memory.notes"], vec!["file.read"]);

    let error = validate_manifest(&manifest).expect_err("legacy capability should fail");

    assert!(error
      .to_string()
      .contains("must use the `<kind>:<identifier>` format"));
  }

  #[test]
  fn build_capability_registry_skips_disabled_and_invalid_plugins() {
    let plugins = vec![
      PluginCatalogEntry {
        id: "workspace-notes".to_string(),
        name: "workspace-notes".to_string(),
        version: "0.1.0".to_string(),
        display_name: "Workspace Notes".to_string(),
        status: "ready".to_string(),
        description: "Enabled plugin".to_string(),
        author_name: Some("Pith".to_string()),
        enabled: true,
        default_enabled: true,
        capabilities: vec![
          "prompt_pack:workspace.notes".to_string(),
          "settings:workspace.preferences".to_string(),
        ],
        permissions: vec!["file.read".to_string(), "file.write".to_string()],
        manifest_path: "plugins/official/workspace-notes/pith-plugin.json".to_string(),
        provenance: "official".to_string(),
        validation_error: None,
      },
      PluginCatalogEntry {
        id: "shell-recorder".to_string(),
        name: "shell-recorder".to_string(),
        version: "0.1.0".to_string(),
        display_name: "Shell Recorder".to_string(),
        status: "ready".to_string(),
        description: "Disabled plugin".to_string(),
        author_name: Some("Pith".to_string()),
        enabled: false,
        default_enabled: false,
        capabilities: vec!["hook:shell.recorder".to_string()],
        permissions: vec!["shell.exec".to_string()],
        manifest_path: "plugins/official/shell-recorder/pith-plugin.json".to_string(),
        provenance: "official".to_string(),
        validation_error: None,
      },
      PluginCatalogEntry {
        id: "broken-plugin".to_string(),
        name: "broken-plugin".to_string(),
        version: "invalid".to_string(),
        display_name: "Broken Plugin".to_string(),
        status: "invalid".to_string(),
        description: "Invalid plugin".to_string(),
        author_name: None,
        enabled: false,
        default_enabled: false,
        capabilities: vec![],
        permissions: vec![],
        manifest_path: "plugins/official/broken/pith-plugin.json".to_string(),
        provenance: "official".to_string(),
        validation_error: Some("plugin capability kind `memory` is not supported".to_string()),
      },
    ];

    let registry = build_capability_registry(&plugins);

    assert_eq!(registry.len(), 2);
    assert_eq!(registry[0].kind, "prompt_pack");
    assert_eq!(registry[1].kind, "settings");
    assert!(registry
      .iter()
      .all(|entry| entry.plugin_id == "workspace-notes"));
  }
}
