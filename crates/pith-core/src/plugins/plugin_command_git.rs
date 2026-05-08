use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{bail, Result};
use pith_process::{
  configure_process_group, join_bounded_pipe_reader, read_bounded_pipe_in_background,
  terminate_process_group_or_child,
};

const GIT_COMMAND_TIMEOUT: Duration = Duration::from_secs(10);
const GIT_COMMAND_POLL_INTERVAL: Duration = Duration::from_millis(50);
const GIT_COMMAND_OUTPUT_LIMIT: usize = 256 * 1024;
const GIT_DIFF_SAFE_FLAGS: [&str; 2] = ["--no-ext-diff", "--no-textconv"];

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
  let stat_args = git_diff_args("--stat");
  let names_args = git_diff_args("--name-only");
  let stat = git_workspace_output(workspace_root, &stat_args)?;
  let names = git_workspace_output(workspace_root, &names_args)?;

  Some(GitDiffSnapshot { stat, names })
}

fn git_diff_args(mode: &'static str) -> Vec<&'static str> {
  let mut args = vec!["diff"];
  args.extend(GIT_DIFF_SAFE_FLAGS);
  args.push(mode);
  args
}

fn git_workspace_output(workspace_root: &Path, args: &[&str]) -> Option<String> {
  let output = run_git_workspace_command(workspace_root, args).ok()?;
  if output.timed_out || !output.success {
    return None;
  }
  Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn run_git_workspace_command(workspace_root: &Path, args: &[&str]) -> Result<GitCommandOutput> {
  let mut child = build_git_command(workspace_root, args)
    .stdin(Stdio::null())
    .stdout(Stdio::piped())
    .stderr(Stdio::null())
    .spawn()?;
  let Some(stdout) = child.stdout.take() else {
    bail!("git stdout pipe was unavailable");
  };
  let stdout_reader = read_bounded_pipe_in_background(stdout, GIT_COMMAND_OUTPUT_LIMIT);

  let started_at = Instant::now();
  let (success, timed_out) = loop {
    if let Some(status) = child.try_wait()? {
      break (status.success(), false);
    }

    if started_at.elapsed() >= GIT_COMMAND_TIMEOUT {
      terminate_git_child(&mut child);
      break (false, true);
    }

    thread::sleep(GIT_COMMAND_POLL_INTERVAL);
  };

  let stdout = join_bounded_pipe_reader(Some(stdout_reader)).bytes;
  Ok(GitCommandOutput {
    stdout,
    success,
    timed_out,
  })
}

fn build_git_command(workspace_root: &Path, args: &[&str]) -> Command {
  let mut process = Command::new("git");
  process.arg("-C").arg(workspace_root).args(args);

  configure_process_group(&mut process);

  process
}

fn terminate_git_child(child: &mut std::process::Child) {
  terminate_process_group_or_child(child, Duration::from_millis(200));
  let _ = child.wait();
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn git_pipe_reader_bounds_retained_output() {
    let input = std::io::Cursor::new(vec![b'x'; GIT_COMMAND_OUTPUT_LIMIT + 512]);
    let output = join_bounded_pipe_reader(Some(read_bounded_pipe_in_background(
      input,
      GIT_COMMAND_OUTPUT_LIMIT,
    )));

    assert_eq!(output.bytes.len(), GIT_COMMAND_OUTPUT_LIMIT);
    assert_eq!(output.source_byte_count, GIT_COMMAND_OUTPUT_LIMIT + 512);
  }

  #[test]
  fn git_diff_args_disable_external_helpers() {
    let args = git_diff_args("--stat");

    assert!(args.contains(&"--no-ext-diff"));
    assert!(args.contains(&"--no-textconv"));
    assert_eq!(args.last(), Some(&"--stat"));
  }
}
