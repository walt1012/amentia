mod notification_loop;
mod request_router;
mod request_supervisor;
mod runtime_io;
mod runtime_lock;

use std::io::{self, BufRead};
use std::sync::{
  atomic::{AtomicBool, Ordering},
  Arc, Mutex,
};

use amentia_core::RuntimeContext;
use amentia_protocol::{JsonRpcRequest, JsonRpcResponse};
use anyhow::Result;
use notification_loop::start_notification_loop;
use request_router::route_runtime_request;
use request_supervisor::RequestSupervisor;
use runtime_io::RuntimeOutput;
use runtime_lock::lock_context;

fn main() -> Result<()> {
  let context = Arc::new(Mutex::new(RuntimeContext::new()?));
  let output = RuntimeOutput::stdout();
  let running = Arc::new(AtomicBool::new(true));
  let mut request_supervisor = RequestSupervisor::new(Arc::clone(&context), output.clone());
  let notification_thread =
    start_notification_loop(Arc::clone(&context), output.clone(), Arc::clone(&running));
  let stdin = io::stdin();

  for line in stdin.lock().lines() {
    let line = line?;
    let trimmed = line.trim();

    if trimmed.is_empty() {
      continue;
    }
    request_supervisor.reap_finished();

    let request = match serde_json::from_str::<JsonRpcRequest>(trimmed) {
      Ok(request) => request,
      Err(error) => {
        let response = JsonRpcResponse::error(serde_json::Value::Null, -32700, error.to_string());
        output.write_json(&response)?;
        continue;
      }
    };

    route_runtime_request(request, &mut request_supervisor, &context, &output)?;
  }

  {
    let mut locked_context = lock_context(&context);
    locked_context.cancel_running_work();
  }
  running.store(false, Ordering::SeqCst);
  let _ = notification_thread.join();
  request_supervisor.join_all();

  Ok(())
}
