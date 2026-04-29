use std::sync::{
  atomic::{AtomicBool, Ordering},
  Arc, Mutex,
};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use pith_core::{collect_notifications, RuntimeContext};

use crate::runtime_io::RuntimeOutput;

pub(crate) fn start_notification_loop(
  context: Arc<Mutex<RuntimeContext>>,
  output: RuntimeOutput,
  running: Arc<AtomicBool>,
) -> JoinHandle<()> {
  thread::spawn(move || {
    while running.load(Ordering::SeqCst) {
      thread::sleep(Duration::from_millis(180));

      let notifications = {
        let Ok(mut locked_context) = context.try_lock() else {
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
