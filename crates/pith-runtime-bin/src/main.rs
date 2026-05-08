mod notification_loop;
mod request_supervisor;
mod runtime_io;
mod runtime_lock;

use std::io::{self, BufRead};
use std::sync::{
  atomic::{AtomicBool, Ordering},
  Arc, Mutex,
};

use anyhow::Result;
use notification_loop::start_notification_loop;
use pith_core::{
  complete_prepared_approval_respond, complete_prepared_plugin_command_run,
  complete_prepared_turn_start, complete_prepared_workspace_search,
  execute_prepared_approval_respond, execute_prepared_plugin_command_run,
  execute_prepared_turn_start, execute_prepared_workspace_search, handle_request,
  prepare_approval_respond, prepare_plugin_command_run, prepare_turn_start,
  prepare_workspace_search, RuntimeContext,
};
use pith_protocol::{methods, JsonRpcRequest, JsonRpcResponse};
use request_supervisor::{RequestLane, RequestSupervisor};
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

    if request.method == methods::TURN_START {
      request_supervisor.spawn_prepared_request(
        RequestLane::LocalExecution,
        request,
        prepare_turn_start,
        execute_prepared_turn_start,
        complete_prepared_turn_start,
      );
      continue;
    }

    if request.method == methods::APPROVAL_RESPOND {
      request_supervisor.spawn_prepared_request(
        RequestLane::LocalExecution,
        request,
        prepare_approval_respond,
        execute_prepared_approval_respond,
        complete_prepared_approval_respond,
      );
      continue;
    }

    if request.method == methods::PLUGIN_COMMAND_RUN {
      request_supervisor.spawn_prepared_request(
        RequestLane::LocalExecution,
        request,
        prepare_plugin_command_run,
        execute_prepared_plugin_command_run,
        complete_prepared_plugin_command_run,
      );
      continue;
    }

    if request.method == methods::WORKSPACE_SEARCH {
      request_supervisor.spawn_prepared_request(
        RequestLane::Independent,
        request,
        prepare_workspace_search,
        execute_prepared_workspace_search,
        |_context, completed| complete_prepared_workspace_search(completed),
      );
      continue;
    }

    let response = {
      let mut locked_context = lock_context(&context);
      handle_request(&mut locked_context, request)
    };

    output.write_json(&response)?;
  }

  running.store(false, Ordering::SeqCst);
  let _ = notification_thread.join();
  request_supervisor.join_all();

  Ok(())
}
