use pith_protocol::{methods, JsonRpcRequest, JsonRpcResponse};

use crate::runtime_context::RuntimeContext;
use crate::{
  approval_requests, memory_requests, model_requests, plugin_commands, plugin_requests,
  runtime_readiness, server_requests, thread_requests, turn_requests, turn_streaming,
  workspace_requests, workspace_search,
};

pub fn handle_request(context: &mut RuntimeContext, request: JsonRpcRequest) -> JsonRpcResponse {
  match request.method.as_str() {
    methods::APPROVAL_RESPOND => approval_requests::handle_approval_respond(context, request),
    methods::INITIALIZE => server_requests::handle_initialize(context, request),
    methods::HEALTH_PING => server_requests::handle_health_ping(request),
    methods::MEMORY_CREATE => memory_requests::handle_memory_create(context, request),
    methods::MEMORY_LIST => memory_requests::handle_memory_list(context, request),
    methods::MEMORY_STATUS => memory_requests::handle_memory_status(context, request),
    methods::MODEL_BOOTSTRAP => model_requests::handle_model_bootstrap(context, request),
    methods::MODEL_HEALTH => model_requests::handle_model_health(context, request),
    methods::PLUGIN_CAPABILITY_REGISTRY => {
      plugin_requests::handle_plugin_capability_registry(context, request)
    }
    methods::PLUGIN_CHANNEL_REGISTRY => {
      plugin_requests::handle_plugin_channel_registry(context, request)
    }
    methods::PLUGIN_COMMAND_REGISTRY => {
      plugin_requests::handle_plugin_command_registry(context, request)
    }
    methods::PLUGIN_COMMAND_RUN => plugin_commands::handle_plugin_command_run(context, request),
    methods::PLUGIN_CONNECTOR_AUTHORIZE => {
      plugin_requests::handle_plugin_connector_authorize(context, request)
    }
    methods::PLUGIN_CONNECTOR_CLEAR_CREDENTIAL => {
      plugin_requests::handle_plugin_connector_clear_credential(context, request)
    }
    methods::PLUGIN_CONNECTOR_REGISTRY => {
      plugin_requests::handle_plugin_connector_registry(context, request)
    }
    methods::PLUGIN_HOOK_REGISTRY => plugin_requests::handle_plugin_hook_registry(context, request),
    methods::PLUGIN_INSPECT => plugin_requests::handle_plugin_inspect(context, request),
    methods::PLUGIN_INSTALL => plugin_requests::handle_plugin_install(context, request),
    methods::PLUGIN_LIST => plugin_requests::handle_plugin_list(context, request),
    methods::PLUGIN_REFRESH => plugin_requests::handle_plugin_refresh(context, request),
    methods::PLUGIN_REMOVE => plugin_requests::handle_plugin_remove(context, request),
    methods::PLUGIN_SET_ENABLED => plugin_requests::handle_plugin_set_enabled(context, request),
    methods::RUNTIME_READINESS => JsonRpcResponse::success(
      request.id,
      &runtime_readiness::build_runtime_readiness(context),
    ),
    methods::WORKSPACE_CURRENT => workspace_requests::handle_workspace_current(context, request),
    methods::WORKSPACE_OPEN => workspace_requests::handle_workspace_open(context, request),
    methods::WORKSPACE_SEARCH => workspace_search::handle_workspace_search(context, request),
    methods::WORKSPACE_SEARCH_CANCEL_RUNNING => {
      workspace_search::handle_workspace_search_cancel_running(context, request)
    }
    methods::TURN_CANCEL => turn_streaming::handle_turn_cancel(context, request),
    methods::TURN_CANCEL_RUNNING => turn_streaming::handle_turn_cancel_running(context, request),
    methods::THREAD_READ => thread_requests::handle_thread_read(context, request),
    methods::THREAD_START => thread_requests::handle_thread_start(context, request),
    methods::THREAD_LIST => thread_requests::handle_thread_list(context, request),
    methods::TURN_START => turn_requests::handle_turn_start(context, request),
    _ => JsonRpcResponse::error(request.id, -32601, "Method not found"),
  }
}
