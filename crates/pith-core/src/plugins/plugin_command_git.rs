use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{bail, Result};

const GIT_COMMAND_TIMEOUT: Duration = Duration::from_secs(10);
const GIT_COMMAND_POLL_INTERVAL: Duration = Duration::from_millis(50);
const GIT_COMMAND_OUTPUT_LIMIT: usize = 256 * 1024;

pub(super) struct GitDiffSnapshot {
  pub(super) stat: String,
  pub(super) names: String,
}

struct GitCommandOutput {
  stdout: Vec<u8>,
  success: bool,
  timed_out: bool,
}

pub(super) fn read_git_diff_snapshot(workspace_root: &Path) -> Option<GitDiffSnapshot> {
  let stat = git_workspace_output(workspace_root, &["diff", "--stat"])?;
  let names = git_workspace_output(workspace_root, &["diff", "--name-only"])?;

  Some(GitDiffSnapshot { stat, names })
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
  let Some(stdout) = child.stdout.take() else {
    bail!("git stdout pipe was unavailable");
  };
  let stdout_reader = read_pipe_in_background(stdout);

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

fn read_pipe_in_background<R>(mut reader: R) -> thread::JoinHandle<Vec<u8>>
where
  R: Read + Send + 'static,
{
  thread::spawn(move || {
    let mut output = Vec::with_capacity(GIT_COMMAND_OUTPUT_LIMIT.min(64 * 1024));
    let mut buffer = [0_u8; 8192];

    while let Ok(bytes_read) = reader.read(&mut buffer) {
      if bytes_read == 0 {
        break;
      }

      let remaining_limit = GIT_COMMAND_OUTPUT_LIMIT.saturating_sub(output.len());
      if remaining_limit > 0 {
        output.extend_from_slice(&buffer[..bytes_read.min(remaining_limit)]);
      }
    }

    output
  })
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn git_pipe_reader_bounds_retained_output() {
    let input = std::io::Cursor::new(vec![b'x'; GIT_COMMAND_OUTPUT_LIMIT + 512]);
    let output = read_pipe_in_background(input).join().expect("pipe reader");

    assert_eq!(output.len(), GIT_COMMAND_OUTPUT_LIMIT);
  }
}
