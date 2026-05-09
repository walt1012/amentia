use std::io::Read;
use std::process::{Child, Command, ExitStatus};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

#[derive(Debug, Default)]
pub struct BoundedPipeOutput {
  pub bytes: Vec<u8>,
  pub source_byte_count: usize,
}

pub fn read_bounded_pipe_in_background<R>(
  mut reader: R,
  max_output_bytes: usize,
) -> JoinHandle<BoundedPipeOutput>
where
  R: Read + Send + 'static,
{
  thread::spawn(move || {
    let mut bytes = Vec::with_capacity(max_output_bytes.min(64 * 1024));
    let mut source_byte_count = 0;
    let mut buffer = [0_u8; 8192];

    while let Ok(bytes_read) = reader.read(&mut buffer) {
      if bytes_read == 0 {
        break;
      }

      source_byte_count += bytes_read;
      let remaining_output = max_output_bytes.saturating_sub(bytes.len());
      if remaining_output > 0 {
        bytes.extend_from_slice(&buffer[..bytes_read.min(remaining_output)]);
      }
    }

    BoundedPipeOutput {
      bytes,
      source_byte_count,
    }
  })
}

pub fn join_bounded_pipe_reader(
  reader: Option<JoinHandle<BoundedPipeOutput>>,
) -> BoundedPipeOutput {
  reader
    .and_then(|handle| handle.join().ok())
    .unwrap_or_default()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChildExitReason {
  Completed,
  TimedOut,
  Cancelled,
}

#[derive(Debug)]
pub struct ChildWaitResult {
  pub status: ExitStatus,
  pub reason: ChildExitReason,
}

pub fn wait_for_child<F>(
  child: &mut Child,
  timeout: Duration,
  poll_interval: Duration,
  termination_grace_period: Duration,
  is_cancelled: F,
) -> std::io::Result<ChildWaitResult>
where
  F: Fn() -> bool,
{
  let started_at = Instant::now();

  loop {
    let child_status = match child.try_wait() {
      Ok(status) => status,
      Err(error) => {
        terminate_process_group_or_child(child, termination_grace_period);
        return Err(error);
      }
    };

    if let Some(status) = child_status {
      return Ok(ChildWaitResult {
        status,
        reason: ChildExitReason::Completed,
      });
    }

    if is_cancelled() {
      terminate_process_group_or_child(child, termination_grace_period);
      return Ok(ChildWaitResult {
        status: child.wait()?,
        reason: ChildExitReason::Cancelled,
      });
    }

    if started_at.elapsed() >= timeout {
      terminate_process_group_or_child(child, termination_grace_period);
      return Ok(ChildWaitResult {
        status: child.wait()?,
        reason: ChildExitReason::TimedOut,
      });
    }

    thread::sleep(poll_interval);
  }
}

#[cfg(unix)]
pub fn configure_process_group(process: &mut Command) {
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

#[cfg(not(unix))]
pub fn configure_process_group(_process: &mut Command) {}

pub fn terminate_process_group_or_child(child: &mut Child, grace_period: Duration) {
  #[cfg(unix)]
  {
    terminate_unix_process_group(child, grace_period);
  }

  #[cfg(not(unix))]
  {
    let _ = child.kill();
  }
}

#[cfg(unix)]
fn terminate_unix_process_group(child: &mut Child, grace_period: Duration) {
  let process_group_id = -(child.id() as i32);
  if !send_unix_signal(process_group_id, SIGTERM) {
    let _ = child.kill();
  }

  thread::sleep(grace_period);

  if matches!(child.try_wait(), Ok(None)) {
    if !send_unix_signal(process_group_id, SIGKILL) {
      let _ = child.kill();
    }
  }
}

#[cfg(unix)]
fn send_unix_signal(pid: i32, signal: i32) -> bool {
  unsafe { kill(pid, signal) == 0 }
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
  use super::*;

  #[test]
  fn bounded_pipe_reader_caps_retained_output() {
    let input = std::io::Cursor::new(vec![b'x'; 4096]);
    let output = join_bounded_pipe_reader(Some(read_bounded_pipe_in_background(input, 128)));

    assert_eq!(output.bytes.len(), 128);
    assert_eq!(output.source_byte_count, 4096);
  }

  #[test]
  fn missing_pipe_reader_returns_empty_output() {
    let output = join_bounded_pipe_reader(None);

    assert!(output.bytes.is_empty());
    assert_eq!(output.source_byte_count, 0);
  }

  #[cfg(unix)]
  #[test]
  fn wait_for_child_reports_completed_exit() {
    let mut command = Command::new("sh");
    configure_process_group(&mut command);
    let mut child = command.args(["-c", "exit 7"]).spawn().expect("spawn child");

    let result = wait_for_child(
      &mut child,
      Duration::from_secs(5),
      Duration::from_millis(10),
      Duration::from_millis(10),
      || false,
    )
    .expect("wait child");

    assert_eq!(result.reason, ChildExitReason::Completed);
    assert_eq!(result.status.code(), Some(7));
  }

  #[cfg(unix)]
  #[test]
  fn wait_for_child_reports_timeout() {
    let mut command = Command::new("sh");
    configure_process_group(&mut command);
    let mut child = command
      .args(["-c", "sleep 5"])
      .spawn()
      .expect("spawn child");

    let result = wait_for_child(
      &mut child,
      Duration::from_millis(30),
      Duration::from_millis(10),
      Duration::from_millis(10),
      || false,
    )
    .expect("wait child");

    assert_eq!(result.reason, ChildExitReason::TimedOut);
  }

  #[cfg(unix)]
  #[test]
  fn wait_for_child_reports_cancellation() {
    let mut command = Command::new("sh");
    configure_process_group(&mut command);
    let mut child = command
      .args(["-c", "sleep 5"])
      .spawn()
      .expect("spawn child");

    let result = wait_for_child(
      &mut child,
      Duration::from_secs(5),
      Duration::from_millis(10),
      Duration::from_millis(10),
      || true,
    )
    .expect("wait child");

    assert_eq!(result.reason, ChildExitReason::Cancelled);
  }
}
