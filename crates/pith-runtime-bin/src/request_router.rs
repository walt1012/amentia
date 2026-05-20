use anyhow::Result;
use pith_core::{
  complete_prepared_approval_respond, complete_prepared_plugin_command_run,
  complete_prepared_plugin_refresh, complete_prepared_turn_start,
  complete_prepared_workspace_search, execute_prepared_approval_respond,
  execute_prepared_plugin_command_run, execute_prepared_plugin_refresh,
  execute_prepared_turn_start, execute_prepared_workspace_search, handle_request,
  prepare_approval_respond, prepare_plugin_command_run, prepare_plugin_refresh, prepare_turn_start,
  prepare_workspace_search,
};
use pith_protocol::{methods, JsonRpcRequest};

use crate::request_supervisor::{RequestLane, RequestSupervisor};
use crate::runtime_io::RuntimeOutput;
use crate::runtime_lock::{lock_context, SharedRuntimeContext};

pub(crate) fn route_runtime_request(
  request: JsonRpcRequest,
  request_supervisor: &mut RequestSupervisor,
  context: &SharedRuntimeContext,
  output: &RuntimeOutput,
) -> Result<()> {
  match request.method.as_str() {
    methods::TURN_START => {
      request_supervisor.spawn_prepared_request(
        RequestLane::LocalExecution,
        request,
        prepare_turn_start,
        execute_prepared_turn_start,
        complete_prepared_turn_start,
      );
    }
    methods::APPROVAL_RESPOND => {
      request_supervisor.spawn_prepared_request(
        RequestLane::LocalExecution,
        request,
        prepare_approval_respond,
        execute_prepared_approval_respond,
        complete_prepared_approval_respond,
      );
    }
    methods::PLUGIN_COMMAND_RUN => {
      request_supervisor.spawn_prepared_request(
        RequestLane::LocalExecution,
        request,
        prepare_plugin_command_run,
        execute_prepared_plugin_command_run,
        complete_prepared_plugin_command_run,
      );
    }
    methods::PLUGIN_REFRESH => {
      request_supervisor.spawn_prepared_request(
        RequestLane::Independent,
        request,
        prepare_plugin_refresh,
        execute_prepared_plugin_refresh,
        complete_prepared_plugin_refresh,
      );
    }
    methods::WORKSPACE_SEARCH => {
      request_supervisor.spawn_prepared_request(
        RequestLane::Independent,
        request,
        prepare_workspace_search,
        execute_prepared_workspace_search,
        complete_prepared_workspace_search,
      );
    }
    _ => {
      let response = {
        let mut locked_context = lock_context(context);
        handle_request(&mut locked_context, request)
      };

      output.write_json(&response)?;
    }
  }

  Ok(())
}
