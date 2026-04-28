use std::io::{self, BufRead, Write};
use std::sync::{
  atomic::{AtomicBool, Ordering},
  Arc, Mutex,
};
use std::thread;
use std::time::Duration;

use anyhow::Result;
use pith_core::{
  collect_notifications, complete_prepared_approval_respond, complete_prepared_plugin_command_run,
  complete_prepared_turn_start, execute_prepared_approval_respond,
  execute_prepared_plugin_command_run, execute_prepared_turn_start, handle_request,
  prepare_approval_respond, prepare_plugin_command_run, prepare_turn_start, RuntimeContext,
};
use pith_protocol::{methods, JsonRpcRequest, JsonRpcResponse};

fn main() -> Result<()> {
  let context = Arc::new(Mutex::new(RuntimeContext::new()?));
  let stdout = Arc::new(Mutex::new(io::stdout()));
  let running = Arc::new(AtomicBool::new(true));
  let local_execution_lane = Arc::new(Mutex::new(()));
  let mut request_threads = vec![];
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

    let request = match serde_json::from_str::<JsonRpcRequest>(trimmed) {
      Ok(request) => request,
      Err(error) => {
        let response = JsonRpcResponse::error(serde_json::Value::Null, -32700, error.to_string());
        write_json_line(&stdout, &response)?;
        continue;
      }
    };

    if request.method == methods::TURN_START {
      request_threads.push(start_turn_request(
        Arc::clone(&context),
        Arc::clone(&stdout),
        Arc::clone(&local_execution_lane),
        request,
      ));
      continue;
    }

    if request.method == methods::APPROVAL_RESPOND {
      request_threads.push(start_approval_request(
        Arc::clone(&context),
        Arc::clone(&stdout),
        Arc::clone(&local_execution_lane),
        request,
      ));
      continue;
    }

    if request.method == methods::PLUGIN_COMMAND_RUN {
      request_threads.push(start_plugin_command_request(
        Arc::clone(&context),
        Arc::clone(&stdout),
        Arc::clone(&local_execution_lane),
        request,
      ));
      continue;
    }

    let response = {
      let mut locked_context = context.lock().expect("runtime context lock");
      handle_request(&mut locked_context, request)
    };

    write_json_line(&stdout, &response)?;
  }

  running.store(false, Ordering::SeqCst);
  let _ = notification_thread.join();
  for request_thread in request_threads {
    let _ = request_thread.join();
  }

  Ok(())
}

fn start_turn_request(
  context: Arc<Mutex<RuntimeContext>>,
  stdout: Arc<Mutex<io::Stdout>>,
  local_execution_lane: Arc<Mutex<()>>,
  request: JsonRpcRequest,
) -> thread::JoinHandle<()> {
  thread::spawn(move || {
    let prepared = {
      let mut locked_context = context.lock().expect("runtime context lock");
      prepare_turn_start(&mut locked_context, request)
    };

    let response = match prepared {
      Ok(prepared) => {
        let _lane = local_execution_lane
          .lock()
          .expect("local execution lane lock");
        let completed = execute_prepared_turn_start(prepared);
        let mut locked_context = context.lock().expect("runtime context lock");
        complete_prepared_turn_start(&mut locked_context, completed)
      }
      Err(response) => response,
    };

    let _ = write_json_line(&stdout, &response);
  })
}

fn start_approval_request(
  context: Arc<Mutex<RuntimeContext>>,
  stdout: Arc<Mutex<io::Stdout>>,
  local_execution_lane: Arc<Mutex<()>>,
  request: JsonRpcRequest,
) -> thread::JoinHandle<()> {
  thread::spawn(move || {
    let prepared = {
      let mut locked_context = context.lock().expect("runtime context lock");
      prepare_approval_respond(&mut locked_context, request)
    };

    let response = match prepared {
      Ok(prepared) => {
        let _lane = local_execution_lane
          .lock()
          .expect("local execution lane lock");
        let completed = execute_prepared_approval_respond(prepared);
        let mut locked_context = context.lock().expect("runtime context lock");
        complete_prepared_approval_respond(&mut locked_context, completed)
      }
      Err(response) => response,
    };

    let _ = write_json_line(&stdout, &response);
  })
}

fn start_plugin_command_request(
  context: Arc<Mutex<RuntimeContext>>,
  stdout: Arc<Mutex<io::Stdout>>,
  local_execution_lane: Arc<Mutex<()>>,
  request: JsonRpcRequest,
) -> thread::JoinHandle<()> {
  thread::spawn(move || {
    let prepared = {
      let mut locked_context = context.lock().expect("runtime context lock");
      prepare_plugin_command_run(&mut locked_context, request)
    };

    let response = match prepared {
      Ok(prepared) => {
        let _lane = local_execution_lane
          .lock()
          .expect("local execution lane lock");
        let completed = execute_prepared_plugin_command_run(prepared);
        let mut locked_context = context.lock().expect("runtime context lock");
        complete_prepared_plugin_command_run(&mut locked_context, completed)
      }
      Err(response) => response,
    };

    let _ = write_json_line(&stdout, &response);
  })
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
        let Ok(mut locked_context) = context.try_lock() else {
          continue;
        };
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
