use pith_protocol::{JsonRpcRequest, JsonRpcResponse};

pub use super::plugin_command_completion::complete_prepared_plugin_command_run;
pub(crate) use super::plugin_command_execution::execute_plugin_command_snapshot;
pub use super::plugin_command_execution::execute_prepared_plugin_command_run;
pub(crate) use super::plugin_command_handoff::ensure_plugin_command_handoff;
pub(crate) use super::plugin_command_memory::capture_plugin_command_output_memory;
pub(crate) use super::plugin_command_preparation::PluginCommandFollowUpRequest;
pub(crate) use super::plugin_command_preparation::prepare_approved_plugin_command_snapshot;
pub(crate) use super::plugin_command_preparation::prepare_plugin_command_follow_up_snapshot;
pub use super::plugin_command_preparation::prepare_plugin_command_run;
pub(crate) use super::plugin_command_preparation::prepare_plugin_command_turn_snapshot;
pub(crate) use super::plugin_command_types::PluginCommandOutput;
pub(crate) use super::plugin_command_types::PluginCommandSnapshot;

use crate::RuntimeContext;

pub(crate) fn handle_plugin_command_run(
  context: &mut RuntimeContext,
  request: JsonRpcRequest,
) -> JsonRpcResponse {
  let prepared = match prepare_plugin_command_run(context, request) {
    Ok(prepared) => prepared,
    Err(response) => return response,
  };
  let completed = execute_prepared_plugin_command_run(prepared);
  complete_prepared_plugin_command_run(context, completed)
}
