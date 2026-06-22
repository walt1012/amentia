use std::collections::HashMap;
use std::path::Path;

use anyhow::Result;

use crate::io::{read_command_manifest, read_hook_manifest, read_manifest};
use crate::types::{PluginCatalogEntry, PluginManifest};

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

  insert_connector_workflow_metadata(&mut metadata_by_capability, &manifest);

  for skill in manifest.skills {
    let mut metadata = HashMap::from([
      ("description".to_string(), skill.description),
      ("path".to_string(), skill.path),
    ]);
    metadata.insert("surface".to_string(), "skill".to_string());
    metadata_by_capability.insert(format!("skill:{}", skill.id), metadata);
  }

  for server in manifest.mcp_servers {
    let server_id = server.id;
    let transport = server.transport.unwrap_or_else(|| "stdio".to_string());
    let command = server.command.and_then(|command| {
      let command = command.trim().to_string();
      if command.is_empty() {
        None
      } else {
        Some(command)
      }
    });
    let mut metadata = HashMap::from([
      ("surface".to_string(), "mcp".to_string()),
      ("transport".to_string(), transport.clone()),
    ]);
    if let Some(command) = command.as_ref() {
      metadata.insert("command".to_string(), command.clone());
    }
    if !server.args.is_empty() {
      metadata.insert("args".to_string(), server.args.join(" "));
    }
    insert_mcp_server_status(&mut metadata, &server_id, command.as_deref());
    metadata_by_capability.insert(format!("mcp_server:{server_id}"), metadata);
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

fn insert_connector_workflow_metadata(
  metadata_by_capability: &mut HashMap<String, HashMap<String, String>>,
  manifest: &PluginManifest,
) {
  let connector_metadata = manifest
    .app_connectors
    .iter()
    .map(|connector| {
      (
        connector.id.as_str(),
        (connector.display_name.as_str(), connector.service.as_str()),
      )
    })
    .collect::<HashMap<_, _>>();

  for workflow in &manifest.connector_workflows {
    let Some((connector_display_name, service)) =
      connector_metadata.get(workflow.connector_id.as_str())
    else {
      continue;
    };
    let mut metadata = HashMap::from([
      ("surface".to_string(), "connector_workflow".to_string()),
      ("displayName".to_string(), workflow.display_name.clone()),
      ("connectorName".to_string(), (*connector_display_name).to_string()),
      ("service".to_string(), (*service).to_string()),
      ("action".to_string(), workflow.action.clone()),
    ]);
    if let Some(max_agent_steps) = workflow.max_agent_steps {
      metadata.insert("maxAgentSteps".to_string(), max_agent_steps.to_string());
    }
    if !workflow.stages.is_empty() {
      metadata.insert("stages".to_string(), workflow.stages.join(", "));
    }
    if !workflow.statuses.is_empty() {
      metadata.insert("statuses".to_string(), workflow.statuses.join(", "));
    }
    metadata_by_capability.insert(format!("connector_workflow:{}", workflow.id), metadata);
  }
}

fn insert_mcp_server_status(
  metadata: &mut HashMap<String, String>,
  server_id: &str,
  command: Option<&str>,
) {
  if command.is_none() {
    metadata.insert("serverStatus".to_string(), "missingCommand".to_string());
    metadata.insert(
      "serverError".to_string(),
      format!("MCP server `{server_id}` requires a command for stdio transport."),
    );
  } else {
    metadata.insert("serverStatus".to_string(), "ready".to_string());
  }
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
