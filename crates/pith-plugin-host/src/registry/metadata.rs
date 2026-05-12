use std::collections::HashMap;
use std::path::Path;

use crate::io::read_manifest;
use crate::types::PluginCatalogEntry;

pub(super) fn plugin_capability_metadata(
  plugin: &PluginCatalogEntry,
) -> HashMap<String, HashMap<String, String>> {
  let Ok(manifest) = read_manifest(Path::new(&plugin.manifest_path)) else {
    return HashMap::new();
  };

  let mut metadata_by_capability = HashMap::new();
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
