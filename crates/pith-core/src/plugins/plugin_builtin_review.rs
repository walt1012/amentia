use std::collections::HashMap;
use std::path::Path;

use pith_plugin_host::PluginCommandEntry as HostPluginCommandEntry;
use pith_protocol::WorkspaceSummary;

use super::plugin_command_git::read_git_diff_snapshot;
use super::plugin_command_text::compact_text_preview;

pub(super) fn build_review_diff_summary_result(
  command: &HostPluginCommandEntry,
  workspace: Option<&WorkspaceSummary>,
  input: Option<&str>,
) -> (String, HashMap<String, String>) {
  if !command
    .permissions
    .iter()
    .any(|permission| permission == "file.read")
  {
    return (
      "This command cannot inspect the workspace because its plugin does not declare `file.read`."
        .to_string(),
      HashMap::new(),
    );
  }
  let Some(workspace) = workspace else {
    return (
      "Open a workspace before inspecting the current diff.".to_string(),
      HashMap::new(),
    );
  };
  let workspace_root = Path::new(&workspace.root_path);

  let content = match read_git_diff_snapshot(workspace_root) {
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
  };
  let attributes = input
    .filter(|input| review_request_wants_saved_summary(input))
    .map(|input| review_write_attributes(workspace, input, &content))
    .unwrap_or_default();

  (content, attributes)
}

fn review_request_wants_saved_summary(input: &str) -> bool {
  let normalized = input.to_ascii_lowercase();
  let save_match = ["save", "write", "record", "store", "capture"]
    .iter()
    .any(|term| normalized.contains(term));
  let artifact_match = ["summary", "review", "handoff", "note", "report"]
    .iter()
    .any(|term| normalized.contains(term));

  save_match && artifact_match
}

fn review_write_attributes(
  workspace: &WorkspaceSummary,
  input: &str,
  review_content: &str,
) -> HashMap<String, String> {
  HashMap::from([
    ("nextAction".to_string(), "write_file".to_string()),
    (
      "nextRelativePath".to_string(),
      ".pith/review-summary.md".to_string(),
    ),
    (
      "nextContent".to_string(),
      review_summary_document(workspace, input, review_content),
    ),
    (
      "reviewApplyMode".to_string(),
      "approvalRequiredWrite".to_string(),
    ),
  ])
}

fn review_summary_document(
  workspace: &WorkspaceSummary,
  input: &str,
  review_content: &str,
) -> String {
  format!(
    "# Pith Review Summary\n\nWorkspace: {}\nRequest: {}\n\n{}\n",
    workspace.display_name,
    input.trim(),
    review_content.trim()
  )
}
