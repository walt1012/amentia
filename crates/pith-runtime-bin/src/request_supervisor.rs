use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

use pith_core::RuntimeContext;
use pith_protocol::{JsonRpcRequest, JsonRpcResponse};

use crate::runtime_io::RuntimeOutput;

type SharedRuntimeContext = Arc<Mutex<RuntimeContext>>;

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
    let context = Arc::clone(&self.context);
    let output = self.output.clone();
    let local_execution_lane = match lane {
      RequestLane::LocalExecution => Some(Arc::clone(&self.local_execution_lane)),
      RequestLane::Independent => None,
    };

    self.handles.push(thread::spawn(move || {
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
