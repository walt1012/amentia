use std::collections::HashMap;

use anyhow::Result;
use pith_plugin_host::PluginCommandEntry as HostPluginCommandEntry;
use pith_protocol::{TimelineItem, WorkspaceSummary};

use crate::runtime_memory::RuntimeMemoryNoteDraft;
use crate::RuntimeContext;

pub(super) fn maybe_capture_plugin_command_memory(
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

fn build_plugin_command_memory_body(
  command: &HostPluginCommandEntry,
  workspace: &WorkspaceSummary,
  input: Option<&str>,
  assistant_content: &str,
) -> String {
  let mut body = format!(
    "Plugin: {} ({})\nCommand: {} ({})\nWorkspace: {} at {}.",
    command.plugin_display_name,
    command.plugin_id,
    command.title,
    command.command_id,
    workspace.display_name,
    workspace.root_path
  );
  if let Some(input) = input {
    body.push_str(&format!("\nCommand input: {input}"));
  }
  body.push_str("\n\nCommand result:\n");
  body.push_str(assistant_content.trim());
  body
}

fn plugin_command_memory_tags(command: &HostPluginCommandEntry) -> Vec<String> {
  let mut tags = vec![
    "plugin".to_string(),
    "command".to_string(),
    command.plugin_id.clone(),
    command.command_id.clone(),
  ];
  for tag in &command.memory_note_tags {
    if !tags.iter().any(|existing| existing == tag) {
      tags.push(tag.clone());
    }
  }
  tags
}

pub(super) fn build_plugin_command_memory_warning_item(
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
