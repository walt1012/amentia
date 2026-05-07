use pith_plugin_host::PluginCommandEntry as HostPluginCommandEntry;
use pith_protocol::WorkspaceSummary;

pub(super) fn build_plugin_command_memory_body(
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
