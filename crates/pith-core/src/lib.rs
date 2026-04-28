use pith_protocol::{methods, JsonRpcRequest, JsonRpcResponse};
pub use request_state::{
  CompletedApprovalRespond, CompletedTurnStart, PreparedApprovalRespond, PreparedTurnStart,
};
pub use runtime_context::RuntimeContext;
use runtime_readiness::build_runtime_readiness;

mod turn {
  pub(crate) mod active_turns;
  pub(crate) mod approval_state;
  pub(crate) mod approval_types;
  pub(crate) mod turn_actions;
  pub(crate) mod turn_streaming;
}
pub(crate) use turn::{active_turns, approval_state, approval_types, turn_actions, turn_streaming};

mod threads {
  pub(crate) mod thread_state;
  pub(crate) mod thread_summary;
}
pub(crate) use threads::{thread_state, thread_summary};

mod context {
  pub(crate) mod context_compaction;
  pub(crate) mod context_state;
  pub(crate) mod intent_inference;
  pub(crate) mod local_responses;
}
pub(crate) use context::{context_compaction, intent_inference, local_responses};

mod plugins {
  pub(crate) mod plugin_catalog_state;
  pub(crate) mod plugin_commands;
  pub(crate) mod plugin_hooks;
  pub(crate) mod plugin_permissions;
  pub(crate) mod plugin_requests;
}
pub(crate) use plugins::{
  plugin_catalog_state, plugin_commands, plugin_hooks, plugin_permissions, plugin_requests,
};

mod requests {
  pub(crate) mod approval_requests;
  pub(crate) mod memory_requests;
  pub(crate) mod model_requests;
  pub(crate) mod request_params;
  pub(crate) mod request_state;
  pub(crate) mod server_requests;
  pub(crate) mod thread_requests;
  pub(crate) mod turn_requests;
  pub(crate) mod workspace_requests;
}
pub(crate) use requests::{
  approval_requests, memory_requests, model_requests, request_params, request_state,
  server_requests, thread_requests, turn_requests, workspace_requests,
};

mod runtime {
  pub(crate) mod protocol_adapters;
  pub(crate) mod runtime_context;
  pub(crate) mod runtime_readiness;
  pub(crate) mod runtime_sequences;
}
pub(crate) use runtime::{protocol_adapters, runtime_context, runtime_readiness, runtime_sequences};

mod support {
  pub(crate) mod text_utils;
}
pub(crate) use support::text_utils;

mod workspace {
  pub(crate) mod workspace_search;
}
pub(crate) use workspace::workspace_search;

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
