use std::collections::HashMap;
use std::path::Path;

use anyhow::Result;

use crate::io::{read_command_manifest, read_hook_manifest, read_manifest};
use crate::types::PluginCatalogEntry;

use super::capability_identifier_is_safe;

pub(super) fn plugin_capability_metadata(
  plugin: &PluginCatalogEntry,
) -> HashMap<String, HashMap<String, String>> {
  let Ok(manifest) = read_manifest(Path::new(&plugin.manifest_path)) else {
    return HashMap::new();
  };
  let plugin_root = Path::new(&plugin.manifest_path).parent();

  let mut metadata_by_capability = HashMap::new();
  if let Some(plugin_root) = plugin_root {
    for capability in &manifest.capabilities {
      let Some((kind, identifier)) = capability.split_once(':') else {
        continue;
      };
      if !capability_identifier_is_safe(identifier) {
        continue;
      }
      if kind == "command" {
        metadata_by_capability.insert(
          capability.clone(),
          definition_metadata(
            plugin_root,
            "command",
            "commands",
            identifier,
            read_command_manifest,
          ),
        );
      } else if kind == "hook" {
        metadata_by_capability.insert(
          capability.clone(),
          definition_metadata(plugin_root, "hook", "hooks", identifier, read_hook_manifest),
        );
      }
    }
  }

  for skill in manifest.skills {
    let mut metadata = HashMap::from([
      ("description".to_string(), skill.description),
      ("path".to_string(), skill.path),
    ]);
    metadata.insert("surface".to_string(), "skill".to_string());
    metadata_by_capability.insert(format!("skill:{}", skill.id), metadata);
  }

  for server in manifest.mcp_servers {
    let mut metadata = HashMap::from([(
      "transport".to_string(),
      server.transport.unwrap_or_else(|| "stdio".to_string()),
    )]);
    metadata.insert("surface".to_string(), "mcp".to_string());
    if let Some(command) = server.command {
      metadata.insert("command".to_string(), command);
    }
    if !server.args.is_empty() {
      metadata.insert("args".to_string(), server.args.join(" "));
    }
    metadata_by_capability.insert(format!("mcp_server:{}", server.id), metadata);
  }

  let auth_policy = manifest.auth_policy;
  for connector in manifest.app_connectors {
    let mut metadata = HashMap::from([
      ("surface".to_string(), "connector".to_string()),
      ("displayName".to_string(), connector.display_name),
      ("service".to_string(), connector.service),
    ]);
    if let Some(homepage) = connector.homepage {
      metadata.insert("homepage".to_string(), homepage);
    }
    if let Some(auth_policy) = auth_policy.as_ref() {
      metadata.insert("authType".to_string(), auth_policy.auth_type.clone());
      metadata.insert("authRequired".to_string(), auth_policy.required.to_string());
      if !auth_policy.scopes.is_empty() {
        metadata.insert("authScopes".to_string(), auth_policy.scopes.join(", "));
      }
      if let Some(credential_store) = auth_policy.credential_store.as_ref() {
        metadata.insert("credentialStore".to_string(), credential_store.clone());
      }
    }
    metadata_by_capability.insert(format!("connector:{}", connector.id), metadata);
  }

  metadata_by_capability
}

fn definition_metadata<T>(
  plugin_root: &Path,
  surface: &str,
  directory: &str,
  identifier: &str,
  read_definition: fn(&Path) -> Result<T>,
) -> HashMap<String, String> {
  let definition_path = plugin_root
    .join(directory)
    .join(format!("{identifier}.json"));
  let mut metadata = HashMap::from([
    ("surface".to_string(), surface.to_string()),
    (
      "definitionPath".to_string(),
      definition_path.display().to_string(),
    ),
  ]);
  match read_definition(&definition_path) {
    Ok(_) => {
      metadata.insert("definitionStatus".to_string(), "ready".to_string());
    }
    Err(error) => {
      let status = if definition_path.exists() {
        "invalid"
      } else {
        "missing"
      };
      metadata.insert("definitionStatus".to_string(), status.to_string());
      metadata.insert("definitionError".to_string(), error.to_string());
    }
  }
  metadata
}
