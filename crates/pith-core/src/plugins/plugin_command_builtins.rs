use pith_memory::MemoryNote;
use pith_plugin_host::PluginCommandEntry as HostPluginCommandEntry;
use pith_protocol::WorkspaceSummary;

use super::plugin_builtin_readme::build_workspace_readme_note_result;
use super::plugin_builtin_review::build_review_diff_summary_result;
use super::plugin_builtin_shell::build_shell_session_summary_result;

pub(super) struct BuiltinPluginCommandResult {
  pub(super) execution_kind: String,
  pub(super) content: String,
}

pub(crate) fn is_supported_builtin_execution(execution_kind: Option<&str>) -> bool {
  matches!(
    execution_kind,
    Some(
      "builtin.workspaceReadmeNote" | "builtin.shellSessionSummary" | "builtin.reviewDiffSummary"
    )
  )
}

pub(super) fn execute_builtin_plugin_command(
  command: &HostPluginCommandEntry,
  workspace: Option<&WorkspaceSummary>,
  input: Option<&str>,
  memory_notes: &[MemoryNote],
) -> std::result::Result<BuiltinPluginCommandResult, (i32, String)> {
  let execution_kind = command.execution_kind.as_deref().ok_or_else(|| {
    (
      -32053,
      format!(
        "Plugin command `{}` requires an explicit execution contract.",
        command.command_id
      ),
    )
  })?;
  let content = match execution_kind {
    "builtin.workspaceReadmeNote" => build_workspace_readme_note_result(command, workspace, input),
    "builtin.shellSessionSummary" => build_shell_session_summary_result(memory_notes, workspace),
    "builtin.reviewDiffSummary" => build_review_diff_summary_result(command, workspace),
    _ => {
      return Err((
        -32053,
        format!(
          "Plugin command `{}` requires an explicit execution contract.",
          command.command_id
        ),
      ))
    }
  };

  Ok(BuiltinPluginCommandResult {
    execution_kind: execution_kind.to_string(),
    content,
  })
}
