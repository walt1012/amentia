use amentia_protocol::WorkspaceSummary;

use super::plugin_hook_types::PluginHookMemoryCapture;

pub(crate) fn build_plugin_hook_memory_body(
  workspace: &WorkspaceSummary,
  capture: &PluginHookMemoryCapture,
) -> String {
  format!(
    "Plugin: {} ({})\nHook: {} ({})\nEvent: {}\nWorkspace: {} at {}.\nCommand: {}\nExit code: {}\n{}\nstdout: {}\nstderr: {}\n\nHook result:\n{}",
    capture.hook.plugin_display_name,
    capture.hook.plugin_id,
    capture.hook.title,
    capture.hook.hook_id,
    capture.hook.event,
    workspace.display_name,
    workspace.root_path,
    capture.command,
    capture.exit_code,
    capture.sandbox.display_line(),
    capture.stdout_preview,
    capture.stderr_preview,
    capture.content
  )
}
