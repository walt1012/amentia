use std::path::Path;

use anyhow::Error;

use crate::io::{read_command_manifest, read_manifest};
use crate::manifest::PluginManifest;
use crate::types::{
  PluginCatalogEntry, PluginCommandEntry, PluginCommandExecutionEntry, PluginConnectorWorkflowEntry,
};

use super::capability_identifier_is_safe;
use super::command_contract::command_execution_entry;
use super::workflow_commands::connector_workflow_command_ids;

pub fn build_command_registry(plugins: &[PluginCatalogEntry]) -> Vec<PluginCommandEntry> {
  let mut commands = vec![];

  for plugin in plugins
    .iter()
    .filter(|plugin| plugin.status == "ready" && plugin.enabled)
  {
    let Some(plugin_root) = Path::new(&plugin.manifest_path).parent() else {
      continue;
    };
    let workflow_entries = read_manifest(Path::new(&plugin.manifest_path))
      .map(|manifest| connector_workflow_entries(plugin, &manifest, plugin_root))
      .unwrap_or_default();

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
      let mut execution = command.execution.as_ref().and_then(command_execution_entry);
      let manifest_error =
        bind_connector_workflow(plugin, identifier, execution.as_mut(), &workflow_entries);
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
        manifest_error,
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

fn bind_connector_workflow(
  plugin: &PluginCatalogEntry,
  identifier: &str,
  execution: Option<&mut PluginCommandExecutionEntry>,
  workflows: &[PluginConnectorWorkflowEntry],
) -> Option<String> {
  let execution = execution?;
  let workflow_id = execution.workflow_id.as_deref()?.trim();
  if workflow_id.is_empty() {
    return None;
  }
  if execution.connector_ids.as_ref().is_none_or(Vec::is_empty) {
    return Some(format!(
      "Plugin command `{identifier}` declares workflow `{workflow_id}` without a connector execution binding."
    ));
  }
  let workflow_capability = format!("connector_workflow:{workflow_id}");
  if !plugin
    .capabilities
    .iter()
    .any(|capability| capability == &workflow_capability)
  {
    return Some(format!(
      "Plugin command `{identifier}` references undeclared connector workflow `{workflow_id}`."
    ));
  }
  let Some(workflow) = workflows
    .iter()
    .find(|workflow| workflow.workflow_id == workflow_id)
  else {
    return Some(format!(
      "Plugin command `{identifier}` references workflow `{workflow_id}` without a connectorWorkflows contract."
    ));
  };
  if !execution.connector_ids.as_ref().is_some_and(|connectors| {
    connectors
      .iter()
      .any(|connector| connector == &workflow.connector_id)
  }) {
    return Some(format!(
      "Plugin command `{identifier}` workflow `{workflow_id}` is not bound to connector `{}`.",
      workflow.connector_id
    ));
  }
  execution.workflow = Some(workflow.clone());

  None
}

fn connector_workflow_entries(
  plugin: &PluginCatalogEntry,
  manifest: &PluginManifest,
  plugin_root: &Path,
) -> Vec<PluginConnectorWorkflowEntry> {
  manifest
    .connector_workflows
    .iter()
    .filter_map(|workflow| {
      let connector = manifest
        .app_connectors
        .iter()
        .find(|connector| connector.id == workflow.connector_id)?;
      Some(PluginConnectorWorkflowEntry {
        workflow_id: workflow.id.clone(),
        display_name: workflow.display_name.clone(),
        connector_id: workflow.connector_id.clone(),
        service: connector.service.clone(),
        action: workflow.action.clone(),
        max_agent_steps: workflow.max_agent_steps,
        stages: workflow.stages.clone(),
        statuses: workflow.statuses.clone(),
        command_ids: connector_workflow_command_ids(plugin, plugin_root, &workflow.id),
      })
    })
    .collect()
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
