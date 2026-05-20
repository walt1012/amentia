use std::path::Path;

use anyhow::Error;

use crate::io::read_command_manifest;
use crate::types::{PluginCatalogEntry, PluginCommandEntry};

use super::capability_identifier_is_safe;
use super::command_contract::command_execution_entry;

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
      if !capability_identifier_is_safe(identifier) {
        continue;
      }

      let command_path = plugin_root
        .join("commands")
        .join(format!("{identifier}.json"));
      let command = match read_command_manifest(&command_path) {
        Ok(command) => command,
        Err(error) => {
          commands.push(invalid_command_entry(
            plugin,
            identifier,
            &command_path,
            error,
          ));
          continue;
        }
      };
      let memory_note_title = command
        .memory
        .as_ref()
        .map(|memory| memory.note_title.clone());
      let execution = command.execution.as_ref().and_then(command_execution_entry);
      let execution_kind = execution.as_ref().map(|execution| execution.kind.clone());
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
        execution,
        execution_kind,
        manifest_error: None,
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

fn invalid_command_entry(
  plugin: &PluginCatalogEntry,
  identifier: &str,
  command_path: &Path,
  error: Error,
) -> PluginCommandEntry {
  PluginCommandEntry {
    command_id: format!("{}::{}", plugin.id, identifier),
    title: identifier.to_string(),
    description: "Plugin command manifest could not be loaded.".to_string(),
    prompt: String::new(),
    plugin_id: plugin.id.clone(),
    plugin_display_name: plugin.display_name.clone(),
    permissions: plugin.permissions.clone(),
    source_path: command_path.display().to_string(),
    execution: None,
    execution_kind: None,
    manifest_error: Some(format!(
      "Plugin command `{}` manifest could not be loaded: {error}",
      identifier
    )),
    memory_note_title: None,
    memory_note_source: None,
    memory_note_tags: vec![],
  }
}
