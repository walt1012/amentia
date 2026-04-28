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
  complete_prepared_turn_start, complete_prepared_workspace_search,
  execute_prepared_approval_respond, execute_prepared_plugin_command_run,
  execute_prepared_turn_start, execute_prepared_workspace_search, handle_request,
  prepare_approval_respond, prepare_plugin_command_run, prepare_turn_start,
  prepare_workspace_search, RuntimeContext,
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
    reap_finished_threads(&mut request_threads);

    let request = match serde_json::from_str::<JsonRpcRequest>(trimmed) {
      Ok(request) => request,
      Err(error) => {
        let response = JsonRpcResponse::error(serde_json::Value::Null, -32700, error.to_string());
        write_json_line(&stdout, &response)?;
        continue;
      }
    };

    if request.method == methods::TURN_START {
      request_threads.push(start_prepared_request(
        Arc::clone(&context),
        Arc::clone(&stdout),
        Some(Arc::clone(&local_execution_lane)),
        request,
        prepare_turn_start,
        execute_prepared_turn_start,
        complete_prepared_turn_start,
      ));
      continue;
    }

    if request.method == methods::APPROVAL_RESPOND {
      request_threads.push(start_prepared_request(
        Arc::clone(&context),
        Arc::clone(&stdout),
        Some(Arc::clone(&local_execution_lane)),
        request,
        prepare_approval_respond,
        execute_prepared_approval_respond,
        complete_prepared_approval_respond,
      ));
      continue;
    }

    if request.method == methods::PLUGIN_COMMAND_RUN {
      request_threads.push(start_prepared_request(
        Arc::clone(&context),
        Arc::clone(&stdout),
        Some(Arc::clone(&local_execution_lane)),
        request,
        prepare_plugin_command_run,
        execute_prepared_plugin_command_run,
        complete_prepared_plugin_command_run,
      ));
      continue;
    }

    if request.method == methods::WORKSPACE_SEARCH {
      request_threads.push(start_prepared_request(
        Arc::clone(&context),
        Arc::clone(&stdout),
        None,
        request,
        prepare_workspace_search,
        execute_prepared_workspace_search,
        |_context, completed| complete_prepared_workspace_search(completed),
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

fn start_prepared_request<Prepared, Completed, Prepare, Execute, Complete>(
  context: Arc<Mutex<RuntimeContext>>,
  stdout: Arc<Mutex<io::Stdout>>,
  local_execution_lane: Option<Arc<Mutex<()>>>,
  request: JsonRpcRequest,
  prepare: Prepare,
  execute: Execute,
  complete: Complete,
) -> thread::JoinHandle<()>
where
  Prepared: Send + 'static,
  Completed: Send + 'static,
  Prepare: FnOnce(&mut RuntimeContext, JsonRpcRequest) -> std::result::Result<Prepared, JsonRpcResponse>
    + Send
    + 'static,
  Execute: FnOnce(Prepared) -> Completed + Send + 'static,
  Complete: FnOnce(&mut RuntimeContext, Completed) -> JsonRpcResponse + Send + 'static,
{
  thread::spawn(move || {
    let prepared = {
      let mut locked_context = context.lock().expect("runtime context lock");
      prepare(&mut locked_context, request)
    };

    let response = match prepared {
      Ok(prepared) => {
        let completed = if let Some(local_execution_lane) = local_execution_lane {
          let _lane = local_execution_lane
            .lock()
            .expect("local execution lane lock");
          execute(prepared)
        } else {
          execute(prepared)
        };
        let mut locked_context = context.lock().expect("runtime context lock");
        complete(&mut locked_context, completed)
      }
      Err(response) => response,
    };

    let _ = write_json_line(&stdout, &response);
  })
}

fn reap_finished_threads(request_threads: &mut Vec<thread::JoinHandle<()>>) {
  let mut index = 0;
  while index < request_threads.len() {
    if request_threads[index].is_finished() {
      let request_thread = request_threads.swap_remove(index);
      let _ = request_thread.join();
    } else {
      index += 1;
    }
  }
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
