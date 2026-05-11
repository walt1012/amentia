use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

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
  handles: Vec<JoinHandle<()>>,
}

impl RequestSupervisor {
  pub(crate) fn new(context: SharedRuntimeContext, output: RuntimeOutput) -> Self {
    Self {
      context,
      output,
      local_execution_lane: Arc::new(Mutex::new(())),
      handles: vec![],
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

    self.handles.push(thread::spawn(move || {
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
        JsonRpcResponse::error(
          request_id,
          -32099,
          "Runtime request recovered after an internal panic",
        )
      });

      let _ = output.write_json(&response);
    }));
  }

  pub(crate) fn reap_finished(&mut self) {
    let mut index = 0;
    while index < self.handles.len() {
      if self.handles[index].is_finished() {
        let handle = self.handles.swap_remove(index);
        let _ = handle.join();
      } else {
        index += 1;
      }
    }
  }

  pub(crate) fn join_all(self) {
    for handle in self.handles {
      let _ = handle.join();
    }
  }
}
