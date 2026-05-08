use std::collections::HashMap;
use std::path::Path;

use crate::io::{read_command_manifest, read_hook_manifest, read_manifest};
use crate::types::{
  PluginCapabilityRegistration, PluginCatalogEntry, PluginCommandEntry, PluginConnectorEntry,
  PluginHookEntry,
};

pub fn build_capability_registry(
  plugins: &[PluginCatalogEntry],
) -> Vec<PluginCapabilityRegistration> {
  let mut registrations = plugins
    .iter()
    .filter(|plugin| plugin.status == "ready" && plugin.enabled)
    .flat_map(|plugin| {
      let metadata_by_capability = plugin_capability_metadata(plugin);
      plugin
        .capabilities
        .iter()
        .filter_map(|capability| {
          let (kind, identifier) = capability.split_once(':')?;
          Some(PluginCapabilityRegistration {
            capability_id: format!("{}::{}", plugin.id, capability),
            kind: kind.to_string(),
            identifier: identifier.to_string(),
            plugin_id: plugin.id.clone(),
            plugin_display_name: plugin.display_name.clone(),
            permissions: plugin.permissions.clone(),
            manifest_path: plugin.manifest_path.clone(),
            metadata: metadata_by_capability
              .get(capability)
              .cloned()
              .unwrap_or_default(),
          })
        })
        .collect::<Vec<_>>()
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

pub fn build_connector_registry(plugins: &[PluginCatalogEntry]) -> Vec<PluginConnectorEntry> {
  let mut connectors = vec![];

  for plugin in plugins.iter().filter(|plugin| plugin.status == "ready") {
    let Ok(manifest) = read_manifest(Path::new(&plugin.manifest_path)) else {
      continue;
    };

    let auth_type = manifest
      .auth_policy
      .as_ref()
      .map(|policy| policy.auth_type.clone());
    let auth_required = manifest
      .auth_policy
      .as_ref()
      .map(|policy| policy.required)
      .unwrap_or(false);
    let auth_scopes = manifest
      .auth_policy
      .as_ref()
      .map(|policy| policy.scopes.clone())
      .unwrap_or_default();
    let credential_store = manifest
      .auth_policy
      .as_ref()
      .and_then(|policy| policy.credential_store.clone());
    let status = if !plugin.enabled {
      "disabled"
    } else if auth_required {
      "needsAuth"
    } else {
      "ready"
    };

    for connector in manifest.app_connectors {
      connectors.push(PluginConnectorEntry {
        connector_id: format!("{}::{}", plugin.id, connector.id),
        display_name: connector.display_name,
        service: connector.service,
        plugin_id: plugin.id.clone(),
        plugin_display_name: plugin.display_name.clone(),
        enabled: plugin.enabled,
        status: status.to_string(),
        permissions: plugin.permissions.clone(),
        manifest_path: plugin.manifest_path.clone(),
        homepage: connector.homepage,
        auth_type: auth_type.clone(),
        auth_required,
        auth_scopes: auth_scopes.clone(),
        credential_store: credential_store.clone(),
      });
    }
  }

  connectors.sort_by(|left, right| {
    left
      .service
      .cmp(&right.service)
      .then_with(|| left.display_name.cmp(&right.display_name))
      .then_with(|| left.connector_id.cmp(&right.connector_id))
  });
  connectors
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
      let memory_note_title = command
        .memory
        .as_ref()
        .map(|memory| memory.note_title.clone());
      let execution_kind = command
        .execution
        .as_ref()
        .map(|execution| execution.kind.clone());
      let memory_note_source = command
        .memory
        .as_ref()
        .and_then(|memory| memory.note_source.clone());
      let memory_note_tags = command
        .memory
        .as_ref()
        .map(|memory| memory.note_tags.clone())
        .unwrap_or_default();

      commands.push(PluginCommandEntry {
        command_id: format!("{}::{}", plugin.id, identifier),
        title: command.title,
        description: command.description,
        prompt: command.prompt,
        plugin_id: plugin.id.clone(),
        plugin_display_name: plugin.display_name.clone(),
        permissions: plugin.permissions.clone(),
        source_path: command_path.display().to_string(),
        execution_kind,
        memory_note_title,
        memory_note_source,
        memory_note_tags,
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
      let memory_note_title = hook.memory.as_ref().map(|memory| memory.note_title.clone());
      let memory_note_source = hook
        .memory
        .as_ref()
        .and_then(|memory| memory.note_source.clone());
      let memory_note_tags = hook
        .memory
        .as_ref()
        .map(|memory| memory.note_tags.clone())
        .unwrap_or_default();

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
        memory_note_title,
        memory_note_source,
        memory_note_tags,
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

fn plugin_capability_metadata(
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
