pub use request_state::{
  CompletedApprovalRespond, CompletedTurnStart, PreparedApprovalRespond, PreparedTurnStart,
};
pub use runtime_context::RuntimeContext;

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
  pub(crate) mod request_dispatch;
  pub(crate) mod request_params;
  pub(crate) mod request_state;
  pub(crate) mod server_requests;
  pub(crate) mod thread_requests;
  pub(crate) mod turn_requests;
  pub(crate) mod workspace_requests;
}
pub(crate) use requests::{
  approval_requests, memory_requests, model_requests, request_dispatch, request_params,
  request_state, server_requests, thread_requests, turn_requests, workspace_requests,
};

mod runtime {
  pub(crate) mod protocol_adapters;
  pub(crate) mod runtime_context;
  pub(crate) mod runtime_readiness;
  pub(crate) mod runtime_sequences;
}
pub(crate) use runtime::{
  protocol_adapters, runtime_context, runtime_readiness, runtime_sequences,
};

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
pub use request_dispatch::handle_request;
pub use turn_requests::{
  complete_prepared_turn_start, execute_prepared_turn_start, prepare_turn_start,
};
pub use turn_streaming::collect_notifications;
pub use workspace_search::{
  complete_prepared_workspace_search, execute_prepared_workspace_search, prepare_workspace_search,
  CompletedWorkspaceSearch, PreparedWorkspaceSearch,
};

#[cfg(test)]
mod tests;
