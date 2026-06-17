use std::collections::HashMap;

use amentia_plugin_host::PluginCommandEntry as HostPluginCommandEntry;
use amentia_protocol::{TimelineItem, WorkspaceSummary};
use anyhow::Result;

use super::plugin_command_memory_body::build_plugin_command_memory_body;
use super::plugin_command_memory_tags::plugin_command_memory_tags;
use super::plugin_command_types::PluginRunnerMemoryNoteDraft as RunnerMemoryNoteDraft;
use crate::runtime_memory::RuntimeMemoryNoteDraft;
use crate::RuntimeContext;

pub(crate) fn capture_plugin_command_output_memory(
  context: &mut RuntimeContext,
  thread_id: &str,
  command: &HostPluginCommandEntry,
  workspace: Option<&WorkspaceSummary>,
  input: Option<&str>,
  items: &[TimelineItem],
  capture_memory: bool,
  runner_memory_notes: &[RunnerMemoryNoteDraft],
) -> Vec<TimelineItem> {
  if !runner_memory_notes.is_empty() {
    return match capture_plugin_runner_memory_notes(
      context,
      thread_id,
      command,
      workspace,
      runner_memory_notes,
    ) {
      Ok(items) => items,
      Err(error) => vec![build_plugin_command_memory_warning_item(
        command,
        error.to_string(),
      )],
    };
  }

  if capture_memory {
    return match maybe_capture_plugin_command_memory(
      context, thread_id, command, input, workspace, items,
    ) {
      Ok(Some(item)) => vec![item],
      Ok(None) => vec![],
      Err(error) => vec![build_plugin_command_memory_warning_item(
        command,
        error.to_string(),
      )],
    };
  }

  vec![]
}

fn maybe_capture_plugin_command_memory(
  context: &mut RuntimeContext,
  thread_id: &str,
  command: &HostPluginCommandEntry,
  input: Option<&str>,
  workspace: Option<&WorkspaceSummary>,
  items: &[TimelineItem],
) -> Result<Option<TimelineItem>> {
  let Some(note_title) = command.memory_note_title.as_ref() else {
    return Ok(None);
  };
  let Some(assistant_message) = items
    .iter()
    .rev()
    .find(|item| item.kind == "assistantMessage")
  else {
    return Ok(None);
  };
  let workspace = workspace
    .cloned()
    .or_else(|| {
      context
        .thread_state
        .find(thread_id)
        .and_then(|thread| thread.workspace_cloned())
    })
    .or_else(|| context.workspace_state.current_cloned());
  let Some(workspace) = workspace else {
    return Ok(None);
  };

  let note_body =
    build_plugin_command_memory_body(command, &workspace, input, &assistant_message.content);
  let note_source = command
    .memory_note_source
    .clone()
    .unwrap_or_else(|| format!("plugin.{}", command.plugin_id));
  let note_tags = plugin_command_memory_tags(command);
  let note = context.create_memory_note(RuntimeMemoryNoteDraft::new(
    note_title.clone(),
    note_body,
    workspace.display_name.clone(),
    note_source,
    note_tags,
  ))?;

  Ok(Some(TimelineItem {
    kind: "system".to_string(),
    title: "Memory Note Saved".to_string(),
    content: format!(
      "Saved workspace memory note \"{}\" from {}.",
      note.title, command.title
    ),
    attributes: Some(HashMap::from([
      ("memoryNoteId".to_string(), note.id),
      ("memoryNoteTitle".to_string(), note.title),
      ("memoryScope".to_string(), note.scope),
      ("pluginId".to_string(), command.plugin_id.clone()),
      ("commandId".to_string(), command.command_id.clone()),
    ])),
  }))
}

fn build_plugin_command_memory_warning_item(
  command: &HostPluginCommandEntry,
  error_message: String,
) -> TimelineItem {
  TimelineItem {
    kind: "warning".to_string(),
    title: "Plugin Memory Capture Failed".to_string(),
    content: format!(
      "{} could not save its workspace memory note. {}",
      command.title, error_message
    ),
    attributes: Some(HashMap::from([
      ("pluginId".to_string(), command.plugin_id.clone()),
      ("commandId".to_string(), command.command_id.clone()),
    ])),
  }
}

fn capture_plugin_runner_memory_notes(
  context: &mut RuntimeContext,
  thread_id: &str,
  command: &HostPluginCommandEntry,
  workspace: Option<&WorkspaceSummary>,
  notes: &[RunnerMemoryNoteDraft],
) -> Result<Vec<TimelineItem>> {
  if notes.is_empty() {
    return Ok(vec![]);
  }
  let workspace = workspace
    .cloned()
    .or_else(|| {
      context
        .thread_state
        .find(thread_id)
        .and_then(|thread| thread.workspace_cloned())
    })
    .or_else(|| context.workspace_state.current_cloned());
  let Some(workspace) = workspace else {
    return Ok(vec![build_plugin_command_memory_warning_item(
      command,
      "Runner memory notes require an open workspace.".to_string(),
    )]);
  };

  let mut items = vec![];
  for note in notes {
    let source = note
      .source
      .clone()
      .unwrap_or_else(|| format!("plugin.{}", command.plugin_id));
    let saved = context.create_memory_note(RuntimeMemoryNoteDraft::new(
      note.title.clone(),
      runner_memory_body(command, &workspace, &note.body),
      workspace.display_name.clone(),
      source,
      runner_memory_tags(command, &note.tags),
    ))?;
    items.push(TimelineItem {
      kind: "system".to_string(),
      title: "Plugin Memory Note Saved".to_string(),
      content: format!(
        "Saved runner memory note \"{}\" from {}.",
        saved.title, command.title
      ),
      attributes: Some(HashMap::from([
        ("memoryNoteId".to_string(), saved.id),
        ("memoryNoteTitle".to_string(), saved.title),
        ("memoryScope".to_string(), saved.scope),
        ("pluginId".to_string(), command.plugin_id.clone()),
        ("commandId".to_string(), command.command_id.clone()),
      ])),
    });
  }

  Ok(items)
}

fn runner_memory_body(
  command: &HostPluginCommandEntry,
  workspace: &WorkspaceSummary,
  body: &str,
) -> String {
  format!(
    "Plugin: {} ({})\nCommand: {} ({})\nWorkspace: {} at {}.\n\nRunner memory note:\n{}",
    command.plugin_display_name,
    command.plugin_id,
    command.title,
    command.command_id,
    workspace.display_name,
    workspace.root_path,
    body.trim()
  )
}

fn runner_memory_tags(command: &HostPluginCommandEntry, note_tags: &[String]) -> Vec<String> {
  let mut tags = plugin_command_memory_tags(command);
  if !tags.iter().any(|tag| tag == "runner") {
    tags.push("runner".to_string());
  }
  for tag in note_tags {
    if !tags.iter().any(|existing| existing == tag) {
      tags.push(tag.clone());
    }
  }

  tags
}
