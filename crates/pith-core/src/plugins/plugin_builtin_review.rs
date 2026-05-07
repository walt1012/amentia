use std::path::Path;

use pith_plugin_host::PluginCommandEntry as HostPluginCommandEntry;
use pith_protocol::WorkspaceSummary;

use super::plugin_command_git::read_git_diff_snapshot;
use super::plugin_command_text::compact_text_preview;

pub(super) fn build_review_diff_summary_result(
  command: &HostPluginCommandEntry,
  workspace: Option<&WorkspaceSummary>,
) -> String {
  if !command
    .permissions
    .iter()
    .any(|permission| permission == "file.read")
  {
    return "This command cannot inspect the workspace because its plugin does not declare `file.read`."
      .to_string();
  }
  let Some(workspace) = workspace else {
    return "Open a workspace before inspecting the current diff.".to_string();
  };
  let workspace_root = Path::new(&workspace.root_path);

  match read_git_diff_snapshot(workspace_root) {
    Some(snapshot) if !snapshot.stat.trim().is_empty() || !snapshot.names.trim().is_empty() => {
      format!(
        "Current diff snapshot for {}.\n\nChanged files:\n{}\n\nDiff stat:\n{}\n\nReview focus:\n- Check behavioral regressions first.\n- Verify missing tests around changed paths.\n- Inspect risky file writes before approving follow-up changes.",
        workspace.display_name,
        compact_text_preview(&snapshot.names, 20, 900),
        compact_text_preview(&snapshot.stat, 20, 1200)
      )
    }
    Some(_) => format!(
      "No active git diff was detected in {}. The review command is ready once files change.",
      workspace.display_name
    ),
    _ => format!(
      "Could not read a git diff in {}. Ensure the workspace is a git repository and git is available.",
      workspace.display_name
    ),
  }
}
