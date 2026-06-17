use std::path::Path;

use amentia_plugin_host::PluginCommandEntry as HostPluginCommandEntry;
use amentia_protocol::WorkspaceSummary;
use amentia_tools::read_file;

use super::plugin_command_text::compact_text_preview;
use crate::turn::turn_tool_limits::READ_FILE_PREVIEW_MAX_BYTES;

pub(super) fn build_workspace_readme_note_result(
  command: &HostPluginCommandEntry,
  workspace: Option<&WorkspaceSummary>,
  input: Option<&str>,
) -> String {
  if !command
    .permissions
    .iter()
    .any(|permission| permission == "file.read")
  {
    return "This command cannot read workspace files because its plugin does not declare `file.read`."
      .to_string();
  }
  let Some(workspace) = workspace else {
    return "Open a project before capturing a project memory note.".to_string();
  };

  match read_file(
    Path::new(&workspace.root_path),
    "README.md",
    READ_FILE_PREVIEW_MAX_BYTES,
  ) {
    Ok(result) => {
      let summary = compact_text_preview(&result.content, 10, 900);
      let input_summary = input
        .map(|value| format!("\nOperator input: {}", value.trim()))
        .unwrap_or_default();
      format!(
        "Workspace note candidate from README.md in {}.{}\n\n{}",
        workspace.display_name, input_summary, summary
      )
    }
    Err(error) => format!(
      "Could not capture a README-based note in {}: {}",
      workspace.display_name, error
    ),
  }
}
