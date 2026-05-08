use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

use anyhow::Result;
use pith_process::{configure_process_group, wait_for_child, ChildExitReason};

use crate::shell_output_artifacts::discard_shell_output_artifact_directory;

const SHELL_COMMAND_TIMEOUT: Duration = Duration::from_secs(120);
const SHELL_POLL_INTERVAL: Duration = Duration::from_millis(50);

pub(crate) fn shell_command_timeout() -> Duration {
  SHELL_COMMAND_TIMEOUT
}

pub(crate) fn shell_command_timeout_seconds() -> u64 {
  SHELL_COMMAND_TIMEOUT.as_secs()
}

pub(crate) fn run_shell_with_timeout(
  command: &str,
  workspace_root: &Path,
  sandbox_policy: &pith_sandbox::SandboxPolicy,
  timeout: Duration,
  max_output_bytes: usize,
  artifact_directory: PathBuf,
) -> Result<ShellOutput> {
  let mut shell_command = build_shell_command(command, workspace_root, sandbox_policy);
  let child_result = shell_command
    .current_dir(workspace_root)
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn();
  let mut child = match child_result {
    Ok(child) => child,
    Err(error) => {
      return discard_artifact_and_return_error(&artifact_directory, error.into());
    }
  };
  let stdout_reader = child.stdout.take().map(|reader| {
    read_pipe_in_background(
      reader,
      artifact_directory.join("stdout.txt"),
      max_output_bytes,
    )
  });
  let stderr_reader = child.stderr.take().map(|reader| {
    read_pipe_in_background(
      reader,
      artifact_directory.join("stderr.txt"),
      max_output_bytes,
    )
  });
  let wait = match wait_for_child(
    &mut child,
    timeout,
    SHELL_POLL_INTERVAL,
    Duration::from_millis(200),
    || false,
  ) {
    Ok(wait) => wait,
    Err(error) => {
      return discard_artifact_and_return_error(&artifact_directory, error.into());
    }
  };
  let timed_out = wait.reason == ChildExitReason::TimedOut;

  let stdout = match join_pipe_reader(stdout_reader) {
    Ok(stdout) => stdout,
    Err(error) => {
      return discard_artifact_and_return_error(&artifact_directory, error);
    }
  };
  let stderr = match join_pipe_reader(stderr_reader) {
    Ok(stderr) => stderr,
    Err(error) => {
      return discard_artifact_and_return_error(&artifact_directory, error);
    }
  };
  let artifact_directory =
    if stdout.source_byte_count > max_output_bytes || stderr.source_byte_count > max_output_bytes {
      Some(artifact_directory)
    } else {
      discard_shell_output_artifact_directory(&artifact_directory);
      None
    };

  Ok(ShellOutput {
    exit_code: if timed_out {
      -1
    } else {
      wait.status.code().unwrap_or(-1)
    },
    stdout: stdout.preview,
    stderr: stderr.preview,
    stdout_source_bytes: stdout.source_byte_count,
    stderr_source_bytes: stderr.source_byte_count,
    artifact_directory,
    timed_out,
  })
}

pub(crate) struct ShellOutput {
  pub(crate) exit_code: i32,
  pub(crate) stdout: Vec<u8>,
  pub(crate) stderr: Vec<u8>,
  pub(crate) stdout_source_bytes: usize,
  pub(crate) stderr_source_bytes: usize,
  pub(crate) artifact_directory: Option<PathBuf>,
  pub(crate) timed_out: bool,
}

struct PipeCapture {
  preview: Vec<u8>,
  source_byte_count: usize,
}

fn read_pipe_in_background<R>(
  mut reader: R,
  artifact_path: PathBuf,
  max_preview_bytes: usize,
) -> thread::JoinHandle<Result<PipeCapture>>
where
  R: Read + Send + 'static,
{
  thread::spawn(move || {
    if let Some(parent) = artifact_path.parent() {
      fs::create_dir_all(parent)?;
    }
    let mut artifact = OpenOptions::new()
      .write(true)
      .create_new(true)
      .open(artifact_path)?;
    let mut preview = Vec::with_capacity(max_preview_bytes.min(64 * 1024));
    let mut source_byte_count = 0;
    let mut buffer = [0_u8; 8192];

    loop {
      let bytes_read = reader.read(&mut buffer)?;
      if bytes_read == 0 {
        break;
      }

      artifact.write_all(&buffer[..bytes_read])?;
      source_byte_count += bytes_read;
      let remaining_preview = max_preview_bytes.saturating_sub(preview.len());
      if remaining_preview > 0 {
        preview.extend_from_slice(&buffer[..bytes_read.min(remaining_preview)]);
      }
    }

    Ok(PipeCapture {
      preview,
      source_byte_count,
    })
  })
}

fn join_pipe_reader(
  reader: Option<thread::JoinHandle<Result<PipeCapture>>>,
) -> Result<PipeCapture> {
  reader
    .map(|handle| match handle.join() {
      Ok(result) => result,
      Err(_) => Ok(PipeCapture {
        preview: vec![],
        source_byte_count: 0,
      }),
    })
    .unwrap_or_else(|| {
      Ok(PipeCapture {
        preview: vec![],
        source_byte_count: 0,
      })
    })
}

fn discard_artifact_and_return_error<T>(
  artifact_directory: &Path,
  error: anyhow::Error,
) -> Result<T> {
  discard_shell_output_artifact_directory(artifact_directory);
  Err(error)
}

#[cfg(target_family = "windows")]
fn build_shell_command(
  command: &str,
  _workspace_root: &Path,
  _sandbox_policy: &pith_sandbox::SandboxPolicy,
) -> Command {
  let mut process = Command::new("powershell");
  process.args(["-NoProfile", "-Command", command]);
  process
}

#[cfg(target_os = "macos")]
fn build_shell_command(
  command: &str,
  _workspace_root: &Path,
  sandbox_policy: &pith_sandbox::SandboxPolicy,
) -> Command {
  if pith_sandbox::native_sandbox_available() {
    let profile = pith_sandbox::macos_seatbelt_profile(sandbox_policy);
    let mut process = Command::new(pith_sandbox::macos_sandbox_exec_path());
    process
      .arg("-p")
      .arg(profile)
      .arg("/bin/sh")
      .arg("-lc")
      .arg(command);
    if let Some(temporary_root) = sandbox_policy.temporary_root() {
      process
        .env("TMPDIR", temporary_root)
        .env("TMP", temporary_root)
        .env("TEMP", temporary_root);
    }
    configure_process_group(&mut process);
    return process;
  }

  build_unix_shell_command(command)
}

#[cfg(all(unix, not(target_os = "macos")))]
fn build_shell_command(
  command: &str,
  _workspace_root: &Path,
  _sandbox_policy: &pith_sandbox::SandboxPolicy,
) -> Command {
  build_unix_shell_command(command)
}

#[cfg(unix)]
fn build_unix_shell_command(command: &str) -> Command {
  let mut process = Command::new("sh");
  process.args(["-lc", command]);
  configure_process_group(&mut process);
  process
}

#[cfg(test)]
mod tests {
  use std::fs;
  use std::path::PathBuf;
  use std::time::{Duration, SystemTime, UNIX_EPOCH};

  use super::*;

  #[cfg(unix)]
  #[test]
  fn shell_timeout_terminates_blocking_command() {
    let workspace = unique_temp_workspace("shell-timeout");
    fs::create_dir_all(&workspace).expect("workspace");

    let artifact_directory = workspace.join("artifacts");
    let result = run_shell_with_timeout(
      "sleep 5",
      &workspace,
      &pith_sandbox::SandboxPolicy::workspace_read_write(&workspace),
      Duration::from_millis(100),
      1024,
      artifact_directory,
    )
    .expect("shell result");

    assert!(result.timed_out);
    assert_eq!(result.exit_code, -1);

    let _ = fs::remove_dir_all(workspace);
  }

  #[test]
  fn pipe_reader_spools_full_output_while_bounding_preview() {
    let workspace = unique_temp_workspace("pipe-spool");
    fs::create_dir_all(&workspace).expect("workspace");
    let artifact_path = workspace.join("stdout.txt");
    let input = std::io::Cursor::new(vec![b'x'; 4096]);

    let capture = join_pipe_reader(Some(read_pipe_in_background(
      input,
      artifact_path.clone(),
      128,
    )))
    .expect("pipe capture");

    assert_eq!(capture.source_byte_count, 4096);
    assert_eq!(capture.preview.len(), 128);
    assert_eq!(fs::read(artifact_path).expect("artifact").len(), 4096);

    let _ = fs::remove_dir_all(workspace);
  }

  #[test]
  fn shell_spawn_failure_discards_empty_artifact_directory() {
    let workspace = unique_temp_workspace("shell-spawn-missing");
    let artifact_directory = unique_temp_workspace("shell-spawn-artifact");
    fs::create_dir_all(&artifact_directory).expect("artifact directory");

    let result = run_shell_with_timeout(
      "printf never-runs",
      &workspace,
      &pith_sandbox::SandboxPolicy::workspace_read_write(&workspace),
      Duration::from_millis(100),
      1024,
      artifact_directory.clone(),
    );

    assert!(result.is_err());
    assert!(!artifact_directory.exists());

    let _ = fs::remove_dir_all(artifact_directory);
  }

  #[test]
  fn artifact_error_boundary_discards_directory() {
    let artifact_directory = unique_temp_workspace("artifact-error-boundary");
    fs::create_dir_all(&artifact_directory).expect("artifact directory");

    let result: Result<()> =
      discard_artifact_and_return_error(&artifact_directory, anyhow::anyhow!("boom"));

    assert!(result.is_err());
    assert!(!artifact_directory.exists());

    let _ = fs::remove_dir_all(artifact_directory);
  }

  fn unique_temp_workspace(prefix: &str) -> PathBuf {
    let nonce = SystemTime::now()
      .duration_since(UNIX_EPOCH)
      .expect("clock")
      .as_nanos();
    std::env::temp_dir().join(format!("pith-tools-{prefix}-{nonce}"))
  }
}
