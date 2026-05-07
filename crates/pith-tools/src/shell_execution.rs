use std::io::Read;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::Result;

#[cfg(target_os = "macos")]
use crate::shell_sandbox::{shell_sandbox_policy, shell_sandbox_temp_root};

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

pub(crate) struct ShellOutput {
  pub(crate) exit_code: i32,
  pub(crate) stdout: Vec<u8>,
  pub(crate) stderr: Vec<u8>,
  pub(crate) timed_out: bool,
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

  fn unique_temp_workspace(prefix: &str) -> PathBuf {
    let nonce = SystemTime::now()
      .duration_since(UNIX_EPOCH)
      .expect("clock")
      .as_nanos();
    std::env::temp_dir().join(format!("pith-tools-{prefix}-{nonce}"))
  }
}
