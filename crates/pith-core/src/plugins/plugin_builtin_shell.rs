use pith_memory::MemoryNote;
use pith_protocol::WorkspaceSummary;

use super::plugin_command_text::compact_text_preview;

pub(super) fn build_shell_session_summary_result(
  memory_notes: &[MemoryNote],
  workspace: Option<&WorkspaceSummary>,
) -> String {
  let workspace_label = workspace
    .map(|workspace| workspace.display_name.as_str())
    .unwrap_or("the current workspace");
  let shell_notes = memory_notes
    .iter()
    .filter(|note| {
      note.source == "plugin.shell-recorder"
        || note.tags.iter().any(|tag| tag == "shell" || tag == "hook")
    })
    .rev()
    .take(5)
    .map(|note| {
      format!(
        "- {}: {}",
        note.title,
        compact_text_preview(&note.body, 2, 220)
      )
    })
    .collect::<Vec<_>>();

  if shell_notes.is_empty() {
    return format!(
      "No shell completion notes are recorded for {} yet. Enable Shell Recorder and approve shell commands to build this timeline.",
      workspace_label
    );
  }

  format!(
    "Recent shell activity for {}:\n{}",
    workspace_label,
    shell_notes.join("\n")
  )
}
