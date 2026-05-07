use std::collections::HashMap;

use anyhow::Result;
use pith_plugin_host::PluginHookEntry as HostPluginHookEntry;
use pith_protocol::{TimelineItem, WorkspaceSummary};

use super::plugin_hook_types::PluginHookMemoryCapture;
use crate::runtime_memory::RuntimeMemoryNoteDraft;
use crate::RuntimeContext;

pub(crate) fn build_plugin_hook_memory_body(
  workspace: &WorkspaceSummary,
  capture: &PluginHookMemoryCapture,
) -> String {
  format!(
    "Plugin: {} ({})\nHook: {} ({})\nEvent: {}\nWorkspace: {} at {}.\nCommand: {}\nExit code: {}\nSandbox: {} via {} ({})\nstdout: {}\nstderr: {}\n\nHook result:\n{}",
    capture.hook.plugin_display_name,
    capture.hook.plugin_id,
    capture.hook.title,
    capture.hook.hook_id,
    capture.hook.event,
    workspace.display_name,
    workspace.root_path,
    capture.command,
    capture.exit_code,
    capture.sandbox.mode,
    capture.sandbox.backend,
    capture.sandbox.state(),
    capture.stdout_preview,
    capture.stderr_preview,
    capture.content
  )
}

pub(crate) fn plugin_hook_memory_tags(hook: &HostPluginHookEntry) -> Vec<String> {
  let mut tags = vec![
    "plugin".to_string(),
    "hook".to_string(),
    hook.plugin_id.clone(),
    hook.hook_id.clone(),
    hook.event.clone(),
  ];
  for tag in &hook.memory_note_tags {
    if !tags.iter().any(|existing| existing == tag) {
      tags.push(tag.clone());
    }
  }
  tags
}

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

#[cfg(test)]
mod tests {
  use super::*;

  fn hook() -> HostPluginHookEntry {
    HostPluginHookEntry {
      hook_id: "shell.recorder".to_string(),
      title: "Record Shell Completion".to_string(),
      description: "Record shell output".to_string(),
      event: "shell.completed".to_string(),
      message_template: "Command {{command}} exited with {{exitCode}}".to_string(),
      plugin_id: "shell-recorder".to_string(),
      plugin_display_name: "Shell Recorder".to_string(),
      permissions: vec!["shell.exec".to_string()],
      source_path: "/tmp/shell-recorder/pith-plugin.json".to_string(),
      memory_note_title: Some("Shell Completion".to_string()),
      memory_note_source: Some("plugin.shell-recorder".to_string()),
      memory_note_tags: vec!["shell".to_string(), "hook".to_string()],
    }
  }

  #[test]
  fn hook_memory_tags_keep_base_tags_and_deduplicate_manifest_tags() {
    let tags = plugin_hook_memory_tags(&hook());

    assert_eq!(
      tags,
      vec![
        "plugin".to_string(),
        "hook".to_string(),
        "shell-recorder".to_string(),
        "shell.recorder".to_string(),
        "shell.completed".to_string(),
        "shell".to_string(),
      ]
    );
  }
}
