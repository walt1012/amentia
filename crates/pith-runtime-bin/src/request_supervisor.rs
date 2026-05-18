use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::{
  atomic::{AtomicBool, Ordering},
  Arc, Mutex, MutexGuard,
};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use pith_core::RuntimeContext;
use pith_protocol::{JsonRpcRequest, JsonRpcResponse};

use crate::runtime_io::RuntimeOutput;
use crate::runtime_lock::{lock_context, SharedRuntimeContext};

#[derive(Debug, Clone, Copy)]
pub(crate) enum RequestLane {
  LocalExecution,
  Independent,
}

pub(crate) struct RequestSupervisor {
  context: SharedRuntimeContext,
  output: RuntimeOutput,
  local_execution_lane: Arc<Mutex<()>>,
  handles: Arc<Mutex<Vec<JoinHandle<()>>>>,
  reaper_running: Arc<AtomicBool>,
  reaper_thread: Option<JoinHandle<()>>,
}

impl RequestSupervisor {
  pub(crate) fn new(context: SharedRuntimeContext, output: RuntimeOutput) -> Self {
    let handles = Arc::new(Mutex::new(vec![]));
    let reaper_running = Arc::new(AtomicBool::new(true));
    let reaper_thread = start_request_reaper(Arc::clone(&handles), Arc::clone(&reaper_running));

    Self {
      context,
      output,
      local_execution_lane: Arc::new(Mutex::new(())),
      handles,
      reaper_running,
      reaper_thread: Some(reaper_thread),
    }
  }

  pub(crate) fn spawn_prepared_request<Prepared, Completed, Prepare, Execute, Complete>(
    &mut self,
    lane: RequestLane,
    request: JsonRpcRequest,
    prepare: Prepare,
    execute: Execute,
    complete: Complete,
  ) where
    Prepared: Send + 'static,
    Completed: Send + 'static,
    Prepare: FnOnce(&mut RuntimeContext, JsonRpcRequest) -> std::result::Result<Prepared, JsonRpcResponse>
      + Send
      + 'static,
    Execute: FnOnce(Prepared) -> Completed + Send + 'static,
    Complete: FnOnce(&mut RuntimeContext, Completed) -> JsonRpcResponse + Send + 'static,
  {
    self.reap_finished();

    let context = Arc::clone(&self.context);
    let output = self.output.clone();
    let local_execution_lane = match lane {
      RequestLane::LocalExecution => Some(Arc::clone(&self.local_execution_lane)),
      RequestLane::Independent => None,
    };

    lock_handles(&self.handles).push(thread::spawn(move || {
      let request_id = request.id.clone();
      let response = catch_unwind(AssertUnwindSafe(|| {
        let prepared = {
          let mut locked_context = lock_context(&context);
          prepare(&mut locked_context, request)
        };

        match prepared {
          Ok(prepared) => {
            let completed = if let Some(local_execution_lane) = local_execution_lane {
              let _lane = local_execution_lane
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
              execute(prepared)
            } else {
              execute(prepared)
            };
            let mut locked_context = lock_context(&context);
            complete(&mut locked_context, completed)
          }
          Err(response) => response,
        }
      }))
      .unwrap_or_else(|_| {
        let recovery_error = {
          let mut locked_context = lock_context(&context);
          locked_context.recover_after_request_panic().err()
        };
        let message = recovery_error
          .map(|error| {
            format!(
              "Runtime request recovered after an internal panic; state cleanup failed: {error}"
            )
          })
          .unwrap_or_else(|| {
            "Runtime request recovered after an internal panic; running work was cleared."
              .to_string()
          });
        JsonRpcResponse::error(request_id, -32099, message)
      });

      let _ = output.write_json(&response);
    }));
  }

  pub(crate) fn reap_finished(&mut self) {
    let mut handles = lock_handles(&self.handles);
    reap_finished_handles(&mut handles);
  }

  pub(crate) fn join_all(mut self) {
    self.reaper_running.store(false, Ordering::SeqCst);
    if let Some(reaper_thread) = self.reaper_thread.take() {
      let _ = reaper_thread.join();
    }

    let handles = {
      let mut handles = lock_handles(&self.handles);
      std::mem::take(&mut *handles)
    };

    for handle in handles {
      let _ = handle.join();
    }
  }
}

fn start_request_reaper(
  handles: Arc<Mutex<Vec<JoinHandle<()>>>>,
  running: Arc<AtomicBool>,
) -> JoinHandle<()> {
  thread::spawn(move || {
    while running.load(Ordering::SeqCst) {
      thread::sleep(Duration::from_millis(500));
      let mut handles = lock_handles(&handles);
      reap_finished_handles(&mut handles);
    }
  })
}

fn lock_handles(handles: &Arc<Mutex<Vec<JoinHandle<()>>>>) -> MutexGuard<'_, Vec<JoinHandle<()>>> {
  handles
    .lock()
    .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn reap_finished_handles(handles: &mut Vec<JoinHandle<()>>) {
  let mut index = 0;
  while index < handles.len() {
    if handles[index].is_finished() {
      let handle = handles.swap_remove(index);
      let _ = handle.join();
    } else {
      index += 1;
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn reap_finished_handles_removes_completed_requests() {
    let mut handles = vec![thread::spawn(|| {})];
    while !handles[0].is_finished() {
      thread::sleep(Duration::from_millis(10));
    }

    reap_finished_handles(&mut handles);

    assert!(handles.is_empty());
  }
}
