use std::collections::HashMap;

use amentia_protocol::{TimelineItem, WorkspaceSummary};
use anyhow::Result;

use super::plugin_hook_memory_body::build_plugin_hook_memory_body;
use super::plugin_hook_memory_tags::plugin_hook_memory_tags;
use super::plugin_hook_types::PluginHookMemoryCapture;
use crate::runtime_memory::RuntimeMemoryNoteDraft;
use crate::RuntimeContext;

pub(crate) fn capture_plugin_hook_memory(
  context: &mut RuntimeContext,
  workspace: &WorkspaceSummary,
  capture: &PluginHookMemoryCapture,
) -> Result<TimelineItem> {
  let Some(note_title) = capture.hook.memory_note_title.as_ref() else {
    return Ok(TimelineItem {
      kind: "system".to_string(),
      title: "Plugin Hook Memory Skipped".to_string(),
      content: format!(
        "{} did not declare a memory note title.",
        capture.hook.title
      ),
      attributes: Some(HashMap::from([(
        "hookId".to_string(),
        capture.hook.hook_id.clone(),
      )])),
    });
  };
  let note_source = capture
    .hook
    .memory_note_source
    .clone()
    .unwrap_or_else(|| format!("plugin.{}", capture.hook.plugin_id));
  let note = context.create_memory_note(RuntimeMemoryNoteDraft::new(
    note_title.clone(),
    build_plugin_hook_memory_body(workspace, capture),
    workspace.display_name.clone(),
    note_source,
    plugin_hook_memory_tags(&capture.hook),
  ))?;

  Ok(TimelineItem {
    kind: "system".to_string(),
    title: "Hook Memory Note Saved".to_string(),
    content: format!(
      "Saved workspace memory note \"{}\" from {}.",
      note.title, capture.hook.title
    ),
    attributes: Some(HashMap::from([
      ("memoryNoteId".to_string(), note.id),
      ("memoryNoteTitle".to_string(), note.title),
      ("memoryScope".to_string(), note.scope),
      ("pluginId".to_string(), capture.hook.plugin_id.clone()),
      ("hookId".to_string(), capture.hook.hook_id.clone()),
    ])),
  })
}
