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
  pub validation_hint: Option<String>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PluginCommandManifest {
  pub title: String,
  pub description: String,
  pub prompt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PluginHookManifest {
  pub title: String,
  pub description: String,
  pub event: String,
  pub message_template: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginCommandEntry {
  pub command_id: String,
  pub title: String,
  pub description: String,
  pub prompt: String,
  pub plugin_id: String,
  pub plugin_display_name: String,
  pub permissions: Vec<String>,
  pub source_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginHookEntry {
  pub hook_id: String,
  pub title: String,
  pub description: String,
  pub event: String,
  pub message_template: String,
  pub plugin_id: String,
  pub plugin_display_name: String,
  pub permissions: Vec<String>,
  pub source_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginRemovalRecord {
  pub plugin_id: String,
  pub display_name: String,
  pub removed_path: String,
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

pub fn configured_plugin_roots() -> Vec<PathBuf> {
  let mut roots = vec![];

  if let Ok(path) = env::var("PITH_PLUGIN_DIR") {
    roots.push(PathBuf::from(path));
    if let Ok(local_path) = env::var("PITH_LOCAL_PLUGIN_DIR") {
      let local_root = PathBuf::from(local_path);
      if !roots.contains(&local_root) {
        roots.push(local_root);
      }
    }
    return roots;
  }

  if let Ok(path) = env::var("PITH_LOCAL_PLUGIN_DIR") {
    roots.push(PathBuf::from(path));
  }

  if let Some(default_root) = default_plugin_root() {
    if !roots.contains(&default_root) {
      roots.push(default_root);
    }
  }

  roots
}

pub fn configured_plugin_install_root() -> PathBuf {
  if let Ok(path) = env::var("PITH_LOCAL_PLUGIN_DIR") {
    return PathBuf::from(path);
  }
  if let Ok(path) = env::var("PITH_PLUGIN_DIR") {
    return PathBuf::from(path);
  }
  default_plugin_root()
    .map(|root| root.join("local"))
    .unwrap_or_else(|| PathBuf::from("plugins").join("local"))
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
  Ok(load_plugin_entry(destination_root.join("pith-plugin.json")))
}

pub fn inspect_plugin_bundle(source_path: &Path) -> Result<PluginCatalogEntry> {
  let source_root = resolve_plugin_source_root(source_path)?;
  let source_manifest_path = source_root.join("pith-plugin.json");
  let manifest = read_manifest(&source_manifest_path)?;
  validate_manifest(&manifest)?;
  Ok(load_plugin_entry(source_manifest_path))
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
  let plugin_entry = load_plugin_entry(resolved_manifest_path.clone());
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
        validation_hint: None,
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
    validation_hint: Some(validation_hint_for_error(&validation_error)),
  }
}

fn validation_hint_for_error(validation_error: &str) -> String {
  if validation_error.contains("failed to parse plugin manifest") {
    return "Check that pith-plugin.json is valid JSON and uses camelCase field names such as `displayName` and `defaultEnabled`."
      .to_string();
  }

  if validation_error.contains("must use the `<kind>:<identifier>` format") {
    return format!(
      "Rewrite each capability as `<kind>:<identifier>`. Supported kinds: {}.",
      KNOWN_CAPABILITY_KINDS.join(", ")
    );
  }

  if validation_error.contains("capability kind") && validation_error.contains("is not supported") {
    return format!(
      "Use one of the supported capability kinds: {}.",
      KNOWN_CAPABILITY_KINDS.join(", ")
    );
  }

  if validation_error.contains("must include a non-empty identifier") {
    return "Add a non-empty identifier after the capability kind, for example `command:workspace.capture-note`."
      .to_string();
  }

  if validation_error.contains("plugin permission") && validation_error.contains("is not supported")
  {
    return format!(
      "Use one of the supported permissions: {}.",
      KNOWN_PERMISSIONS.join(", ")
    );
  }

  "Review the manifest schema, then fix the reported field or value and reload the plugin catalog."
    .to_string()
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

pub fn build_command_registry(plugins: &[PluginCatalogEntry]) -> Vec<PluginCommandEntry> {
  let mut commands = vec![];

  for plugin in plugins
    .iter()
    .filter(|plugin| plugin.status == "ready" && plugin.enabled)
  {
    let Some(plugin_root) = Path::new(&plugin.manifest_path).parent() else {
      continue;
    };

    for capability in &plugin.capabilities {
      let Some((kind, identifier)) = capability.split_once(':') else {
        continue;
      };
      if kind != "command" {
        continue;
      }

      let command_path = plugin_root
        .join("commands")
        .join(format!("{identifier}.json"));
      let Ok(command) = read_command_manifest(&command_path) else {
        continue;
      };

      commands.push(PluginCommandEntry {
        command_id: format!("{}::{}", plugin.id, identifier),
        title: command.title,
        description: command.description,
        prompt: command.prompt,
        plugin_id: plugin.id.clone(),
        plugin_display_name: plugin.display_name.clone(),
        permissions: plugin.permissions.clone(),
        source_path: command_path.display().to_string(),
      });
    }
  }

  commands.sort_by(|left, right| {
    left
      .plugin_display_name
      .cmp(&right.plugin_display_name)
      .then_with(|| left.title.cmp(&right.title))
      .then_with(|| left.command_id.cmp(&right.command_id))
  });
  commands
}

pub fn build_hook_registry(plugins: &[PluginCatalogEntry]) -> Vec<PluginHookEntry> {
  let mut hooks = vec![];

  for plugin in plugins
    .iter()
    .filter(|plugin| plugin.status == "ready" && plugin.enabled)
  {
    let Some(plugin_root) = Path::new(&plugin.manifest_path).parent() else {
      continue;
    };

    for capability in &plugin.capabilities {
      let Some((kind, identifier)) = capability.split_once(':') else {
        continue;
      };
      if kind != "hook" {
        continue;
      }

      let hook_path = plugin_root.join("hooks").join(format!("{identifier}.json"));
      let Ok(hook) = read_hook_manifest(&hook_path) else {
        continue;
      };

      hooks.push(PluginHookEntry {
        hook_id: format!("{}::{}", plugin.id, identifier),
        title: hook.title,
        description: hook.description,
        event: hook.event,
        message_template: hook.message_template,
        plugin_id: plugin.id.clone(),
        plugin_display_name: plugin.display_name.clone(),
        permissions: plugin.permissions.clone(),
        source_path: hook_path.display().to_string(),
      });
    }
  }

  hooks.sort_by(|left, right| {
    left
      .event
      .cmp(&right.event)
      .then_with(|| left.plugin_display_name.cmp(&right.plugin_display_name))
      .then_with(|| left.title.cmp(&right.title))
      .then_with(|| left.hook_id.cmp(&right.hook_id))
  });
  hooks
}

fn read_manifest(path: &Path) -> Result<PluginManifest> {
  let content =
    fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
  serde_json::from_str(&content)
    .with_context(|| format!("failed to parse plugin manifest {}", path.display()))
}

fn read_command_manifest(path: &Path) -> Result<PluginCommandManifest> {
  let content =
    fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
  serde_json::from_str(&content)
    .with_context(|| format!("failed to parse plugin command {}", path.display()))
}

fn read_hook_manifest(path: &Path) -> Result<PluginHookManifest> {
  let content =
    fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
  serde_json::from_str(&content)
    .with_context(|| format!("failed to parse plugin hook {}", path.display()))
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
  use std::time::{SystemTime, UNIX_EPOCH};

  fn create_temp_plugin_root(label: &str) -> PathBuf {
    let unique = SystemTime::now()
      .duration_since(UNIX_EPOCH)
      .expect("system time")
      .as_nanos();
    let path = env::temp_dir().join(format!("pith-plugin-host-{label}-{unique}"));
    fs::create_dir_all(&path).expect("create temp plugin root");
    path
  }

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
        validation_hint: None,
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
        validation_hint: None,
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
        validation_hint: Some(
          "Use one of the supported capability kinds: command, agent, prompt_pack, hook, tool, mcp_server, settings."
            .to_string(),
        ),
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

  #[test]
  fn build_command_registry_loads_enabled_plugin_commands() {
    let plugin_root = create_temp_plugin_root("command-registry");
    let plugin_dir = plugin_root.join("workspace-notes");
    let commands_dir = plugin_dir.join("commands");
    fs::create_dir_all(&commands_dir).expect("create commands dir");
    fs::write(
      plugin_dir.join("pith-plugin.json"),
      r#"{
  "name": "workspace-notes",
  "version": "0.1.0",
  "displayName": "Workspace Notes",
  "description": "Test plugin",
  "author": { "name": "Pith" },
  "capabilities": [
    "command:workspace.capture-note",
    "prompt_pack:workspace.notes"
  ],
  "permissions": [
    "file.read",
    "file.write"
  ],
  "defaultEnabled": true
}"#,
    )
    .expect("write plugin manifest");
    fs::write(
      commands_dir.join("workspace.capture-note.json"),
      r#"{
  "title": "Capture Workspace Note",
  "description": "Prepare a reusable note from the current workspace.",
  "prompt": "Read README.md and summarize the most reusable workspace detail."
}"#,
    )
    .expect("write command definition");

    let plugins = discover_plugins(&plugin_root).expect("discover plugins");
    let commands = build_command_registry(&plugins);

    fs::remove_dir_all(&plugin_root).expect("cleanup plugin root");

    assert_eq!(commands.len(), 1);
    assert_eq!(commands[0].plugin_id, "workspace-notes");
    assert_eq!(commands[0].title, "Capture Workspace Note");
    assert!(commands[0]
      .source_path
      .ends_with("workspace.capture-note.json"));
  }

  #[test]
  fn build_hook_registry_loads_enabled_plugin_hooks() {
    let plugin_root = create_temp_plugin_root("hook-registry");
    let plugin_dir = plugin_root.join("shell-recorder");
    let hooks_dir = plugin_dir.join("hooks");
    fs::create_dir_all(&hooks_dir).expect("create hooks dir");
    fs::write(
      plugin_dir.join("pith-plugin.json"),
      r#"{
  "name": "shell-recorder",
  "version": "0.1.0",
  "displayName": "Shell Recorder",
  "description": "Test plugin",
  "author": { "name": "Pith" },
  "capabilities": [
    "hook:shell.recorder",
    "tool:shell.timeline"
  ],
  "permissions": [
    "shell.exec"
  ],
  "defaultEnabled": true
}"#,
    )
    .expect("write plugin manifest");
    fs::write(
      hooks_dir.join("shell.recorder.json"),
      r#"{
  "title": "Record Shell Completion",
  "description": "Capture a compact shell completion note in the thread timeline.",
  "event": "shell.completed",
  "messageTemplate": "Hook observed {{command}} in {{workspaceName}}."
}"#,
    )
    .expect("write hook definition");

    let plugins = discover_plugins(&plugin_root).expect("discover plugins");
    let hooks = build_hook_registry(&plugins);

    fs::remove_dir_all(&plugin_root).expect("cleanup plugin root");

    assert_eq!(hooks.len(), 1);
    assert_eq!(hooks[0].plugin_id, "shell-recorder");
    assert_eq!(hooks[0].event, "shell.completed");
    assert!(hooks[0].source_path.ends_with("shell.recorder.json"));
  }

  #[test]
  fn discover_plugins_in_roots_merges_local_and_bundled_catalogs() {
    let official_root = create_temp_plugin_root("discover-multi-official");
    let local_root = create_temp_plugin_root("discover-multi-local");
    let official_plugin_dir = official_root.join("official").join("workspace-notes");
    let local_plugin_dir = local_root.join("focus-review");
    fs::create_dir_all(&official_plugin_dir).expect("create official plugin dir");
    fs::create_dir_all(&local_plugin_dir).expect("create local plugin dir");
    fs::write(
      official_plugin_dir.join("pith-plugin.json"),
      r#"{
  "name": "workspace-notes",
  "version": "0.1.0",
  "displayName": "Workspace Notes",
  "description": "Official plugin",
  "author": { "name": "Pith" },
  "capabilities": ["prompt_pack:workspace.notes"],
  "permissions": ["file.read"],
  "defaultEnabled": true
}"#,
    )
    .expect("write official manifest");
    fs::write(
      local_plugin_dir.join("pith-plugin.json"),
      r#"{
  "name": "focus-review",
  "version": "0.1.0",
  "displayName": "Focus Review",
  "description": "Local plugin",
  "author": { "name": "Pith" },
  "capabilities": ["command:review.focus"],
  "permissions": ["file.read"],
  "defaultEnabled": false
}"#,
    )
    .expect("write local manifest");

    let plugins = discover_plugins_in_roots(&[official_root.clone(), local_root.clone()])
      .expect("discover plugins across roots");

    fs::remove_dir_all(&official_root).expect("cleanup official plugin root");
    fs::remove_dir_all(&local_root).expect("cleanup local plugin root");

    assert_eq!(plugins.len(), 2);
    assert!(plugins.iter().any(|plugin| plugin.id == "workspace-notes"));
    assert!(plugins.iter().any(|plugin| plugin.id == "focus-review"));
  }

  #[test]
  fn install_and_remove_local_plugin_bundle_round_trip() {
    let source_root = create_temp_plugin_root("install-source");
    let install_root = create_temp_plugin_root("install-target");
    let source_plugin_dir = source_root.join("focus-review");
    fs::create_dir_all(source_plugin_dir.join("commands")).expect("create source commands dir");
    fs::write(
      source_plugin_dir.join("pith-plugin.json"),
      r#"{
  "name": "focus-review",
  "version": "0.1.0",
  "displayName": "Focus Review",
  "description": "Local plugin",
  "author": { "name": "Pith" },
  "capabilities": ["command:review.focus"],
  "permissions": ["file.read"],
  "defaultEnabled": true
}"#,
    )
    .expect("write source manifest");
    fs::write(
      source_plugin_dir.join("commands/review.focus.json"),
      r#"{
  "title": "Focus Review",
  "description": "Prepare a focused review summary.",
  "prompt": "Review the latest diff and keep the summary focused."
}"#,
    )
    .expect("write command definition");

    let installed_plugin =
      install_plugin_bundle(&source_plugin_dir, &install_root).expect("install plugin bundle");
    let installed_manifest = PathBuf::from(&installed_plugin.manifest_path);
    let removed_plugin =
      remove_local_plugin_bundle(&installed_manifest, &install_root).expect("remove plugin");

    fs::remove_dir_all(&source_root).expect("cleanup source root");
    fs::remove_dir_all(&install_root).expect("cleanup install root");

    assert_eq!(installed_plugin.id, "focus-review");
    assert_eq!(installed_plugin.provenance, "local");
    assert_eq!(removed_plugin.plugin_id, "focus-review");
    assert!(removed_plugin.removed_path.ends_with("focus-review"));
  }

  #[test]
  fn official_plugin_manifests_match_runtime_schema() {
    let official_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../plugins/official");
    let manifests = [
      official_root.join("workspace-notes/pith-plugin.json"),
      official_root.join("shell-recorder/pith-plugin.json"),
      official_root.join("review-assistant/pith-plugin.json"),
    ];

    for manifest_path in manifests {
      let manifest = read_manifest(&manifest_path).expect("parse official manifest");
      validate_manifest(&manifest).expect("validate official manifest");
      assert!(!manifest.display_name.trim().is_empty());
    }

    let hook_manifest =
      read_hook_manifest(&official_root.join("shell-recorder/hooks/shell.recorder.json"))
        .expect("parse official hook manifest");
    assert_eq!(hook_manifest.event, "shell.completed");
    assert!(!hook_manifest.message_template.trim().is_empty());
  }

  #[test]
  fn validation_hint_describes_supported_capability_kinds() {
    let hint = validation_hint_for_error("plugin capability kind `memory` is not supported");

    assert!(hint.contains("supported capability kinds"));
    assert!(hint.contains("command"));
    assert!(hint.contains("settings"));
  }
}
