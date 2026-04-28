use std::collections::HashMap;

use anyhow::Result;
use pith_plugin_host::{
  build_hook_registry, PluginCatalogEntry, PluginHookEntry as HostPluginHookEntry,
};
use pith_protocol::{TimelineItem, WorkspaceSummary};
use pith_tools::{ShellCommandResult, ShellSandboxSummary};

use super::RuntimeContext;

#[derive(Debug, Clone)]
pub(crate) struct PluginHookMemoryCapture {
  pub(crate) hook: HostPluginHookEntry,
  pub(crate) content: String,
  pub(crate) command: String,
  pub(crate) exit_code: i32,
  pub(crate) sandbox: ShellSandboxSummary,
  pub(crate) stdout_preview: String,
  pub(crate) stderr_preview: String,
}

pub(crate) fn build_shell_completed_hook_items(
  plugins: &[PluginCatalogEntry],
  workspace: &WorkspaceSummary,
  command: &str,
  result: &ShellCommandResult,
) -> (Vec<TimelineItem>, Vec<PluginHookMemoryCapture>) {
  let stdout_preview = shell_output_preview(&result.stdout);
  let stderr_preview = shell_output_preview(&result.stderr);
  let mut items = vec![];
  let mut memory_captures = vec![];

  for hook in build_hook_registry(plugins)
    .into_iter()
    .filter(|hook| hook.event == "shell.completed")
  {
    let content = render_hook_message(
      &hook.message_template,
      &[
        ("workspaceName", workspace.display_name.clone()),
        ("command", command.to_string()),
        ("exitCode", result.exit_code.to_string()),
        ("stdoutPreview", stdout_preview.clone()),
        ("stderrPreview", stderr_preview.clone()),
      ],
    );
    if hook.memory_note_title.is_some() {
      memory_captures.push(PluginHookMemoryCapture {
        hook: hook.clone(),
        content: content.clone(),
        command: command.to_string(),
        exit_code: result.exit_code,
        sandbox: result.sandbox.clone(),
        stdout_preview: stdout_preview.clone(),
        stderr_preview: stderr_preview.clone(),
      });
    }
    let mut attributes = result.sandbox.attributes();
    attributes.extend(HashMap::from([
      ("hookId".to_string(), hook.hook_id),
      ("hookEvent".to_string(), hook.event),
      ("pluginId".to_string(), hook.plugin_id),
      ("command".to_string(), command.to_string()),
      ("exitCode".to_string(), result.exit_code.to_string()),
      ("sourcePath".to_string(), hook.source_path),
    ]));
    items.push(TimelineItem {
      kind: "pluginHook".to_string(),
      title: hook.title,
      content,
      attributes: Some(attributes),
    });
  }

  (items, memory_captures)
}

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
  let note = context.create_memory_note(
    note_title.clone(),
    build_plugin_hook_memory_body(workspace, capture),
    workspace.display_name.clone(),
    note_source,
    plugin_hook_memory_tags(&capture.hook),
  )?;

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

fn shell_output_preview(output: &str) -> String {
  let preview = output
    .lines()
    .find(|line| !line.trim().is_empty())
    .unwrap_or(output)
    .trim();

  if preview.is_empty() {
    "none".to_string()
  } else {
    preview.chars().take(120).collect()
  }
}

fn render_hook_message(template: &str, replacements: &[(&str, String)]) -> String {
  let mut rendered = template.to_string();
  for (key, value) in replacements {
    rendered = rendered.replace(&format!("{{{{{key}}}}}"), value);
  }
  rendered
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
  fn shell_output_preview_uses_first_non_empty_line() {
    assert_eq!(
      shell_output_preview("\n\n  first line\nsecond line"),
      "first line"
    );
    assert_eq!(shell_output_preview("   \n\t"), "none");
  }

  #[test]
  fn hook_message_renderer_replaces_declared_tokens() {
    let rendered = render_hook_message(
      "{{workspaceName}} ran {{command}}",
      &[
        ("workspaceName", "pith".to_string()),
        ("command", "git status".to_string()),
      ],
    );

    assert_eq!(rendered, "pith ran git status");
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
