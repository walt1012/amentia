use std::path::Path;

use anyhow::{bail, Context, Result};

use crate::paths::canonical_workspace_root;
use crate::shell_execution::{run_shell_with_timeout, shell_command_timeout};
use crate::shell_sandbox::{
  prepare_shell_sandbox_environment, shell_sandbox_status as build_shell_sandbox_status,
  shell_sandbox_summary as build_shell_sandbox_summary,
};
use crate::types::{ShellCommandResult, ShellSandboxSummary};

pub fn shell_command_timeout_seconds() -> u64 {
  crate::shell_execution::shell_command_timeout_seconds()
}

pub fn run_shell(
  workspace_root: &Path,
  command: &str,
  max_output_bytes: usize,
) -> Result<ShellCommandResult> {
  let workspace_root = canonical_workspace_root(workspace_root)?;
  let trimmed_command = command.trim();
  if trimmed_command.is_empty() {
    bail!("shell command must not be empty");
  }
  let sandbox = shell_sandbox_summary(&workspace_root);
  prepare_shell_sandbox_environment(&workspace_root, &sandbox)?;

  let output = run_shell_with_timeout(trimmed_command, &workspace_root, shell_command_timeout())
    .with_context(|| {
      format!(
        "failed to run shell command in {}",
        workspace_root.display()
      )
    })?;

  let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
  let mut stderr = String::from_utf8_lossy(&output.stderr).into_owned();
  if output.timed_out {
    let timeout_message = format!(
      "Command timed out after {} seconds and was terminated.",
      shell_command_timeout_seconds()
    );
    if stderr.trim().is_empty() {
      stderr = timeout_message;
    } else {
      stderr = format!("{stderr}\n{timeout_message}");
    }
  }
  let combined_len = stdout.len() + stderr.len();
  let was_truncated = combined_len > max_output_bytes * 2;

  Ok(ShellCommandResult {
    command: trimmed_command.to_string(),
    exit_code: output.exit_code,
    stdout: truncate_output(&stdout, max_output_bytes),
    stderr: truncate_output(&stderr, max_output_bytes),
    was_truncated,
    timed_out: output.timed_out,
    sandbox,
  })
}

pub fn shell_sandbox_summary(workspace_root: &Path) -> ShellSandboxSummary {
  build_shell_sandbox_summary(workspace_root)
}

pub fn shell_sandbox_status(workspace_root: &Path) -> pith_sandbox::NativeSandboxStatus {
  build_shell_sandbox_status(workspace_root)
}

fn truncate_output(output: &str, max_output_bytes: usize) -> String {
  let mut collected = String::new();

  for character in output.chars() {
    if collected.len() + character.len_utf8() > max_output_bytes {
      break;
    }
    collected.push(character);
  }

  collected
}

#[cfg(test)]
mod tests {
  use std::fs;
  use std::path::PathBuf;
  use std::time::{SystemTime, UNIX_EPOCH};

  use super::*;

  #[cfg(unix)]
  #[test]
  fn shell_result_reports_sandbox_summary() {
    let workspace = unique_temp_workspace("shell-sandbox");
    fs::create_dir_all(&workspace).expect("workspace");

    let result = run_shell(&workspace, "printf pith", 1024).expect("shell result");

    assert_eq!(result.stdout, "pith");
    assert_eq!(result.sandbox.mode, "workspaceReadWrite");
    assert!(!result.sandbox.backend.is_empty());
    let expected_temp_root = crate::shell_sandbox::shell_sandbox_temp_root(&workspace)
      .display()
      .to_string();
    if result.sandbox.active {
      assert_eq!(
        result.sandbox.temporary_root.as_deref(),
        Some(expected_temp_root.as_str())
      );
    } else {
      assert_eq!(result.sandbox.temporary_root, None);
    }
    assert!(!result.sandbox.detail.is_empty());

    let _ = fs::remove_dir_all(workspace);
  }

  fn unique_temp_workspace(prefix: &str) -> PathBuf {
    let nonce = SystemTime::now()
      .duration_since(UNIX_EPOCH)
      .expect("clock")
      .as_nanos();
    std::env::temp_dir().join(format!("pith-tools-{prefix}-{nonce}"))
  }
}
