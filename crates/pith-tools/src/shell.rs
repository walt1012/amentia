use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};

use crate::paths::canonical_workspace_root;
use crate::types::{ShellCommandResult, ShellSandboxSummary};

const SHELL_COMMAND_TIMEOUT: Duration = Duration::from_secs(120);
const SHELL_POLL_INTERVAL: Duration = Duration::from_millis(50);
const SHELL_SANDBOX_TEMP_DIR: &str = ".pith/sandbox-tmp";

pub fn shell_command_timeout_seconds() -> u64 {
  SHELL_COMMAND_TIMEOUT.as_secs()
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

  let output = run_shell_with_timeout(trimmed_command, &workspace_root, SHELL_COMMAND_TIMEOUT)
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
      SHELL_COMMAND_TIMEOUT.as_secs()
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
  let status = shell_sandbox_status(workspace_root);

  ShellSandboxSummary {
    mode: status.mode,
    backend: status.backend,
    active: status.active,
    temporary_root: status.temporary_root,
    detail: status.detail,
  }
}

pub fn shell_sandbox_status(workspace_root: &Path) -> pith_sandbox::NativeSandboxStatus {
  let policy = shell_sandbox_policy(workspace_root);
  pith_sandbox::native_sandbox_status(&policy)
}

fn shell_sandbox_policy(workspace_root: &Path) -> pith_sandbox::SandboxPolicy {
  pith_sandbox::SandboxPolicy::workspace_read_write(workspace_root)
    .with_temporary_root(shell_sandbox_temp_root(workspace_root))
}

fn shell_sandbox_temp_root(workspace_root: &Path) -> PathBuf {
  workspace_root.join(SHELL_SANDBOX_TEMP_DIR)
}

fn prepare_shell_sandbox_environment(
  workspace_root: &Path,
  sandbox: &ShellSandboxSummary,
) -> Result<()> {
  if sandbox.active {
    let temporary_root = shell_sandbox_temp_root(workspace_root);
    fs::create_dir_all(&temporary_root).with_context(|| {
      format!(
        "failed to create sandbox temporary directory {}",
        temporary_root.display()
      )
    })?;
  }

  Ok(())
}

fn run_shell_with_timeout(
  command: &str,
  workspace_root: &Path,
  timeout: Duration,
) -> Result<ShellOutput> {
  let mut child = build_shell_command(command, workspace_root)
    .current_dir(workspace_root)
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn()?;
  let stdout_reader = child.stdout.take().map(read_pipe_in_background);
  let stderr_reader = child.stderr.take().map(read_pipe_in_background);
  let started_at = Instant::now();
  let mut timed_out = false;

  let status = loop {
    if let Some(status) = child.try_wait()? {
      break status;
    }

    if started_at.elapsed() >= timeout {
      timed_out = true;
      terminate_shell_child(&mut child);
      break child.wait()?;
    }

    thread::sleep(SHELL_POLL_INTERVAL);
  };

  Ok(ShellOutput {
    exit_code: if timed_out {
      -1
    } else {
      status.code().unwrap_or(-1)
    },
    stdout: join_pipe_reader(stdout_reader),
    stderr: join_pipe_reader(stderr_reader),
    timed_out,
  })
}

struct ShellOutput {
  exit_code: i32,
  stdout: Vec<u8>,
  stderr: Vec<u8>,
  timed_out: bool,
}

fn read_pipe_in_background<R>(mut reader: R) -> thread::JoinHandle<Vec<u8>>
where
  R: Read + Send + 'static,
{
  thread::spawn(move || {
    let mut bytes = vec![];
    let _ = reader.read_to_end(&mut bytes);
    bytes
  })
}

fn join_pipe_reader(reader: Option<thread::JoinHandle<Vec<u8>>>) -> Vec<u8> {
  reader
    .and_then(|handle| handle.join().ok())
    .unwrap_or_default()
}

fn terminate_shell_child(child: &mut Child) {
  #[cfg(unix)]
  {
    terminate_unix_process_group(child);
  }

  #[cfg(not(unix))]
  {
    let _ = child.kill();
  }
}

#[cfg(unix)]
fn terminate_unix_process_group(child: &mut Child) {
  let process_group_id = -(child.id() as i32);
  unsafe {
    kill(process_group_id, SIGTERM);
  }
  thread::sleep(Duration::from_millis(200));
  if matches!(child.try_wait(), Ok(None)) {
    unsafe {
      kill(process_group_id, SIGKILL);
    }
  }
}

#[cfg(target_family = "windows")]
fn build_shell_command(command: &str, _workspace_root: &Path) -> Command {
  let mut process = Command::new("powershell");
  process.args(["-NoProfile", "-Command", command]);
  process
}

#[cfg(target_os = "macos")]
fn build_shell_command(command: &str, workspace_root: &Path) -> Command {
  if pith_sandbox::native_sandbox_available() {
    let policy = shell_sandbox_policy(workspace_root);
    let profile = pith_sandbox::macos_seatbelt_profile(&policy);
    let temporary_root = shell_sandbox_temp_root(workspace_root);
    let mut process = Command::new(pith_sandbox::macos_sandbox_exec_path());
    process
      .arg("-p")
      .arg(profile)
      .arg("/bin/sh")
      .arg("-lc")
      .arg(command)
      .env("TMPDIR", &temporary_root)
      .env("TMP", &temporary_root)
      .env("TEMP", &temporary_root);
    set_unix_process_group(&mut process);
    return process;
  }

  build_unix_shell_command(command)
}

#[cfg(all(unix, not(target_os = "macos")))]
fn build_shell_command(command: &str, _workspace_root: &Path) -> Command {
  build_unix_shell_command(command)
}

#[cfg(unix)]
fn build_unix_shell_command(command: &str) -> Command {
  let mut process = Command::new("sh");
  process.args(["-lc", command]);
  set_unix_process_group(&mut process);
  process
}

#[cfg(unix)]
fn set_unix_process_group(process: &mut Command) {
  use std::os::unix::process::CommandExt;

  unsafe {
    process.pre_exec(|| {
      if setpgid(0, 0) == 0 {
        Ok(())
      } else {
        Err(std::io::Error::last_os_error())
      }
    });
  }
}

#[cfg(unix)]
const SIGTERM: i32 = 15;
#[cfg(unix)]
const SIGKILL: i32 = 9;

#[cfg(unix)]
extern "C" {
  fn kill(pid: i32, sig: i32) -> i32;
  fn setpgid(pid: i32, pgid: i32) -> i32;
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
  use std::time::{Duration, SystemTime, UNIX_EPOCH};

  use super::*;

  #[cfg(unix)]
  #[test]
  fn shell_timeout_terminates_blocking_command() {
    let workspace = unique_temp_workspace("shell-timeout");
    fs::create_dir_all(&workspace).expect("workspace");

    let result = run_shell_with_timeout("sleep 5", &workspace, Duration::from_millis(100))
      .expect("shell result");

    assert!(result.timed_out);
    assert_eq!(result.exit_code, -1);

    let _ = fs::remove_dir_all(workspace);
  }

  #[cfg(unix)]
  #[test]
  fn shell_result_reports_sandbox_summary() {
    let workspace = unique_temp_workspace("shell-sandbox");
    fs::create_dir_all(&workspace).expect("workspace");

    let result = run_shell(&workspace, "printf pith", 1024).expect("shell result");

    assert_eq!(result.stdout, "pith");
    assert_eq!(result.sandbox.mode, "workspaceReadWrite");
    assert!(!result.sandbox.backend.is_empty());
    let expected_temp_root = workspace.join(SHELL_SANDBOX_TEMP_DIR).display().to_string();
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
