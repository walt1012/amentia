use std::collections::HashMap;

use pith_plugin_host::{build_hook_registry, PluginCatalogEntry};
use pith_protocol::{TimelineItem, WorkspaceSummary};
use pith_tools::ShellCommandResult;

use super::plugin_hook_shell_preview::shell_output_preview;
use super::plugin_hook_template::render_hook_message;
use super::plugin_hook_types::PluginHookMemoryCapture;

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
    attributes.extend(result.output_context.attributes());
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
