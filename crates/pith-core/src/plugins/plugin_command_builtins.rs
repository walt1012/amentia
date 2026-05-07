use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{bail, Result};
use pith_memory::MemoryNote;
use pith_plugin_host::PluginCommandEntry as HostPluginCommandEntry;
use pith_protocol::WorkspaceSummary;
use pith_tools::read_file;

use super::plugin_command_text::compact_text_preview;

const GIT_COMMAND_TIMEOUT: Duration = Duration::from_secs(10);
const GIT_COMMAND_POLL_INTERVAL: Duration = Duration::from_millis(50);

pub(super) struct BuiltinPluginCommandResult {
  pub(super) execution_kind: String,
  pub(super) content: String,
}

struct GitCommandOutput {
  stdout: Vec<u8>,
  success: bool,
  timed_out: bool,
}

pub(super) fn is_supported_builtin_execution(execution_kind: Option<&str>) -> bool {
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
    "builtin.workspaceReadmeNote" => {
      build_workspace_readme_note_result(command, workspace, input)
    }
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

fn build_workspace_readme_note_result(
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
    return "Open a workspace before capturing a workspace note.".to_string();
  };

  match read_file(Path::new(&workspace.root_path), "README.md", 4096) {
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

fn build_shell_session_summary_result(
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

fn build_review_diff_summary_result(
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
  let stat = git_workspace_output(workspace_root, &["diff", "--stat"]);
  let names = git_workspace_output(workspace_root, &["diff", "--name-only"]);

  match (stat, names) {
    (Some(stat), Some(names)) if !stat.trim().is_empty() || !names.trim().is_empty() => {
      format!(
        "Current diff snapshot for {}.\n\nChanged files:\n{}\n\nDiff stat:\n{}\n\nReview focus:\n- Check behavioral regressions first.\n- Verify missing tests around changed paths.\n- Inspect risky file writes before approving follow-up changes.",
        workspace.display_name,
        compact_text_preview(&names, 20, 900),
        compact_text_preview(&stat, 20, 1200)
      )
    }
    (Some(_), Some(_)) => format!(
      "No active git diff was detected in {}. The review command is ready once files change.",
      workspace.display_name
    ),
    _ => format!(
      "Could not read a git diff in {}. Ensure the workspace is a git repository and git is available.",
      workspace.display_name
    ),
  }
}

fn git_workspace_output(workspace_root: &Path, args: &[&str]) -> Option<String> {
  let output = run_git_workspace_command(workspace_root, args).ok()?;
  if output.timed_out || !output.success {
    return None;
  }
  Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn run_git_workspace_command(workspace_root: &Path, args: &[&str]) -> Result<GitCommandOutput> {
  let mut child = Command::new("git")
    .arg("-C")
    .arg(workspace_root)
    .args(args)
    .stdout(Stdio::piped())
    .stderr(Stdio::null())
    .spawn()?;
  let Some(mut stdout) = child.stdout.take() else {
    bail!("git stdout pipe was unavailable");
  };
  let stdout_reader = thread::spawn(move || {
    let mut buffer = vec![];
    let _ = stdout.read_to_end(&mut buffer);
    buffer
  });

  let started_at = Instant::now();
  let (success, timed_out) = loop {
    if let Some(status) = child.try_wait()? {
      break (status.success(), false);
    }

    if started_at.elapsed() >= GIT_COMMAND_TIMEOUT {
      let _ = child.kill();
      let _ = child.wait();
      break (false, true);
    }

    thread::sleep(GIT_COMMAND_POLL_INTERVAL);
  };

  let stdout = stdout_reader.join().unwrap_or_default();
  Ok(GitCommandOutput {
    stdout,
    success,
    timed_out,
  })
}
