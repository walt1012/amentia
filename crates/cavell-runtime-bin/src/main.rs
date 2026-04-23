use std::io::{self, BufRead, Write};
use std::sync::{
  atomic::{AtomicBool, Ordering},
  Arc, Mutex,
};
use std::thread;
use std::time::Duration;

use anyhow::Result;
use cavell_core::{collect_notifications, handle_request, RuntimeContext};
use cavell_protocol::{JsonRpcNotification, JsonRpcRequest, JsonRpcResponse};

fn main() -> Result<()> {
  let context = Arc::new(Mutex::new(RuntimeContext::new()?));
  let stdout = Arc::new(Mutex::new(io::stdout()));
  let running = Arc::new(AtomicBool::new(true));
  let notification_thread = start_notification_loop(
    Arc::clone(&context),
    Arc::clone(&stdout),
    Arc::clone(&running),
  );
  let stdin = io::stdin();

  for line in stdin.lock().lines() {
    let line = line?;
    let trimmed = line.trim();

    if trimmed.is_empty() {
      continue;
    }

    let response = {
      let mut locked_context = context.lock().expect("runtime context lock");
      match serde_json::from_str::<JsonRpcRequest>(trimmed) {
        Ok(request) => handle_request(&mut locked_context, request),
        Err(error) => JsonRpcResponse::error(serde_json::Value::Null, -32700, error.to_string()),
      }
    };

    write_json_line(&stdout, &response)?;
  }

  running.store(false, Ordering::SeqCst);
  let _ = notification_thread.join();

  Ok(())
}

fn start_notification_loop(
  context: Arc<Mutex<RuntimeContext>>,
  stdout: Arc<Mutex<io::Stdout>>,
  running: Arc<AtomicBool>,
) -> thread::JoinHandle<()> {
  thread::spawn(move || {
    while running.load(Ordering::SeqCst) {
      thread::sleep(Duration::from_millis(180));

      let notifications = {
        let mut locked_context = context.lock().expect("runtime context lock");
        collect_notifications(&mut locked_context)
      };

      let Ok(notifications) = notifications else {
        continue;
      };

      for notification in notifications {
        if write_json_line(&stdout, &notification).is_err() {
          return;
        }
      }
    }
  })
}

fn write_json_line<T: serde::Serialize>(
  stdout: &Arc<Mutex<io::Stdout>>,
  payload: &T,
) -> Result<()> {
  let mut locked_stdout = stdout.lock().expect("stdout lock");
  writeln!(locked_stdout, "{}", serde_json::to_string(payload)?)?;
  locked_stdout.flush()?;
  Ok(())
}
