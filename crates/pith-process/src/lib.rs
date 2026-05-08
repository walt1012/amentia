use std::process::{Child, Command};
use std::thread;
use std::time::Duration;

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
  unsafe {
    kill(process_group_id, SIGTERM);
  }
  thread::sleep(grace_period);
  if matches!(child.try_wait(), Ok(None)) {
    unsafe {
      kill(process_group_id, SIGKILL);
    }
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
