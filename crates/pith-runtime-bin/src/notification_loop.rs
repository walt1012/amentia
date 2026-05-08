use std::sync::{
  atomic::{AtomicBool, Ordering},
  Arc,
};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use pith_core::collect_notifications;

use crate::runtime_io::RuntimeOutput;
use crate::runtime_lock::{try_lock_context, SharedRuntimeContext};

pub(crate) fn start_notification_loop(
  context: SharedRuntimeContext,
  output: RuntimeOutput,
  running: Arc<AtomicBool>,
) -> JoinHandle<()> {
  thread::spawn(move || {
    while running.load(Ordering::SeqCst) {
      thread::sleep(Duration::from_millis(180));

      let notifications = {
        let Some(mut locked_context) = try_lock_context(&context) else {
          continue;
        };
        collect_notifications(&mut locked_context)
      };

      let Ok(notifications) = notifications else {
        continue;
      };

      for notification in notifications {
        if output.write_json(&notification).is_err() {
          return;
        }
      }
    }
  })
}
