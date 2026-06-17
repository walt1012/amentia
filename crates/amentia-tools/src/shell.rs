use std::path::Path;

use anyhow::{bail, Context, Result};

use crate::paths::canonical_workspace_root;
use crate::shell_execution::{run_shell_with_timeout, shell_command_timeout};
use crate::shell_output_artifacts::shell_output_artifact_directory;
use crate::shell_output_context::{
  build_shell_output_context, ShellOutputArtifactContext, ShellOutputContextInput,
};
use crate::shell_sandbox::{
  prepare_shell_sandbox_environment, shell_sandbox_plan,
  shell_sandbox_status as build_shell_sandbox_status,
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
  run_shell_with_cancellation(workspace_root, command, max_output_bytes, || false)
}

pub fn run_shell_with_cancellation<F>(
  workspace_root: &Path,
  command: &str,
  max_output_bytes: usize,
  is_cancelled: F,
) -> Result<ShellCommandResult>
where
  F: Fn() -> bool,
{
  let workspace_root = canonical_workspace_root(workspace_root)?;
  let trimmed_command = command.trim();
  if trimmed_command.is_empty() {
    bail!("shell command must not be empty");
  }
  let sandbox_plan = shell_sandbox_plan(&workspace_root);
  let sandbox = sandbox_plan.summary;
  prepare_shell_sandbox_environment(&workspace_root, &sandbox)?;

  let output = run_shell_with_timeout(
    trimmed_command,
    &workspace_root,
    &sandbox_plan.policy,
    shell_command_timeout(),
    max_output_bytes,
    shell_output_artifact_directory()?,
    is_cancelled,
  )
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
  if output.cancelled {
    let cancellation_message = "Command was cancelled and terminated.";
    if stderr.trim().is_empty() {
      stderr = cancellation_message.to_string();
    } else {
      stderr = format!("{stderr}\n{cancellation_message}");
    }
  }
  let stdout_source_bytes = output.stdout_source_bytes.max(stdout.len());
  let stderr_source_bytes = output.stderr_source_bytes.max(stderr.len());
  let output_context = build_shell_output_context(ShellOutputContextInput {
    stdout: &stdout,
    stderr: &stderr,
    source_stdout_bytes: stdout_source_bytes,
    source_stderr_bytes: stderr_source_bytes,
    budget_bytes: max_output_bytes,
    artifact: ShellOutputArtifactContext {
      stdout_bytes: output.stdout_artifact_bytes,
      stderr_bytes: output.stderr_artifact_bytes,
      max_bytes_per_stream: output.artifact_max_bytes_per_stream,
      directory: output.artifact_directory.as_deref(),
    },
    command: trimmed_command,
  });

  Ok(ShellCommandResult {
    command: trimmed_command.to_string(),
    exit_code: output.exit_code,
    stdout: output_context.stdout_preview,
    stderr: output_context.stderr_preview,
    was_truncated: output_context.context.was_compacted,
    timed_out: output.timed_out,
    cancelled: output.cancelled,
    sandbox,
    output_context: output_context.context,
  })
}

pub fn shell_sandbox_summary(workspace_root: &Path) -> ShellSandboxSummary {
  let workspace_root =
    canonical_workspace_root(workspace_root).unwrap_or_else(|_| workspace_root.to_path_buf());
  build_shell_sandbox_summary(&workspace_root)
}

pub fn shell_sandbox_status(workspace_root: &Path) -> amentia_sandbox::NativeSandboxStatus {
  let workspace_root =
    canonical_workspace_root(workspace_root).unwrap_or_else(|_| workspace_root.to_path_buf());
  build_shell_sandbox_status(&workspace_root)
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

    let result = run_shell(&workspace, "printf amentia", 1024).expect("shell result");

    assert_eq!(result.stdout, "amentia");
    assert_eq!(result.sandbox.mode, "workspaceReadWrite");
    assert!(!result.sandbox.backend.is_empty());
    assert!(!result.sandbox.network_allowed);
    let expected_temp_root = crate::shell_sandbox::shell_sandbox_temp_root(&workspace)
      .display()
      .to_string();
    assert_eq!(
      result.sandbox.temporary_root.as_deref(),
      Some(expected_temp_root.as_str())
    );
    assert!(result
      .sandbox
      .writable_roots
      .contains(&workspace.display().to_string()));
    assert!(result.sandbox.writable_roots.contains(&expected_temp_root));
    assert!(!result.sandbox.detail.is_empty());
    assert_eq!(result.output_context.mode, "sandboxOutputPreview");
    assert!(!result.output_context.was_compacted);

    let _ = fs::remove_dir_all(workspace);
  }

  fn unique_temp_workspace(prefix: &str) -> PathBuf {
    let nonce = SystemTime::now()
      .duration_since(UNIX_EPOCH)
      .expect("clock")
      .as_nanos();
    std::env::temp_dir().join(format!("amentia-tools-{prefix}-{nonce}"))
  }
}
