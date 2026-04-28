use pith_protocol::{methods, JsonRpcRequest, JsonRpcResponse};
pub use request_state::{
  CompletedApprovalRespond, CompletedTurnStart, PreparedApprovalRespond, PreparedTurnStart,
};
pub use runtime_context::RuntimeContext;
use runtime_readiness::build_runtime_readiness;

mod active_turns;
mod approval_requests;
mod approval_state;
mod approval_types;
mod context_compaction;
mod context_state;
mod intent_inference;
mod local_responses;
mod memory_requests;
mod model_requests;
mod plugin_catalog_state;
mod plugin_commands;
mod plugin_hooks;
mod plugin_permissions;
mod plugin_requests;
mod protocol_adapters;
mod request_params;
mod request_state;
mod runtime_context;
mod runtime_readiness;
mod server_requests;
mod text_utils;
mod thread_requests;
mod thread_state;
mod thread_summary;
mod turn_actions;
mod turn_requests;
mod turn_streaming;
mod workspace_requests;
mod workspace_search;

pub use approval_requests::{
  complete_prepared_approval_respond, execute_prepared_approval_respond, prepare_approval_respond,
};
pub use plugin_commands::{
  complete_prepared_plugin_command_run, execute_prepared_plugin_command_run,
  prepare_plugin_command_run, CompletedPluginCommandRun, PreparedPluginCommandRun,
};
pub use turn_requests::{
  complete_prepared_turn_start, execute_prepared_turn_start, prepare_turn_start,
};
pub use turn_streaming::collect_notifications;
pub use workspace_search::{
  complete_prepared_workspace_search, execute_prepared_workspace_search, prepare_workspace_search,
  CompletedWorkspaceSearch, PreparedWorkspaceSearch,
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
    methods::PLUGIN_COMMAND_REGISTRY => {
      plugin_requests::handle_plugin_command_registry(context, request)
    }
    methods::PLUGIN_COMMAND_RUN => plugin_commands::handle_plugin_command_run(context, request),
    methods::PLUGIN_CONNECTOR_REGISTRY => {
      plugin_requests::handle_plugin_connector_registry(context, request)
    }
    methods::PLUGIN_HOOK_REGISTRY => plugin_requests::handle_plugin_hook_registry(context, request),
    methods::PLUGIN_INSTALL => plugin_requests::handle_plugin_install(context, request),
    methods::PLUGIN_LIST => plugin_requests::handle_plugin_list(context, request),
    methods::PLUGIN_REMOVE => plugin_requests::handle_plugin_remove(context, request),
    methods::PLUGIN_SET_ENABLED => plugin_requests::handle_plugin_set_enabled(context, request),
    methods::RUNTIME_READINESS => {
      JsonRpcResponse::success(request.id, &build_runtime_readiness(context))
    }
    methods::WORKSPACE_CURRENT => workspace_requests::handle_workspace_current(context, request),
    methods::WORKSPACE_OPEN => workspace_requests::handle_workspace_open(context, request),
    methods::WORKSPACE_SEARCH => workspace_search::handle_workspace_search(context, request),
    methods::TURN_CANCEL => turn_streaming::handle_turn_cancel(context, request),
    methods::THREAD_READ => thread_requests::handle_thread_read(context, request),
    methods::THREAD_START => thread_requests::handle_thread_start(context, request),
    methods::THREAD_LIST => thread_requests::handle_thread_list(context, request),
    methods::TURN_START => turn_requests::handle_turn_start(context, request),
    _ => JsonRpcResponse::error(request.id, -32601, "Method not found"),
  }
}

#[cfg(test)]
mod tests;
