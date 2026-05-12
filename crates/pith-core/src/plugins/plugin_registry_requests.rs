use pith_protocol::{JsonRpcRequest, JsonRpcResponse, PluginListResult};

use crate::protocol_adapters::{
  build_protocol_capability_registry, build_protocol_command_registry,
  build_protocol_connector_registry, build_protocol_hook_registry, to_protocol_plugin,
};
use crate::RuntimeContext;

pub(crate) fn handle_plugin_capability_registry(
  context: &RuntimeContext,
  request: JsonRpcRequest,
) -> JsonRpcResponse {
  JsonRpcResponse::success(
    request.id,
    &build_protocol_capability_registry(context.plugin_state.catalog()),
  )
}

pub(crate) fn handle_plugin_command_registry(
  context: &RuntimeContext,
  request: JsonRpcRequest,
) -> JsonRpcResponse {
  JsonRpcResponse::success(
    request.id,
    &build_protocol_command_registry(context.plugin_state.catalog()),
  )
}

pub(crate) fn handle_plugin_connector_registry(
  context: &RuntimeContext,
  request: JsonRpcRequest,
) -> JsonRpcResponse {
  JsonRpcResponse::success(
    request.id,
    &build_protocol_connector_registry(&context.plugin_state),
  )
}

pub(crate) fn handle_plugin_hook_registry(
  context: &RuntimeContext,
  request: JsonRpcRequest,
) -> JsonRpcResponse {
  JsonRpcResponse::success(
    request.id,
    &build_protocol_hook_registry(context.plugin_state.catalog()),
  )
}

pub(crate) fn handle_plugin_list(
  context: &RuntimeContext,
  request: JsonRpcRequest,
) -> JsonRpcResponse {
  JsonRpcResponse::success(
    request.id,
    &PluginListResult {
      plugins: context
        .plugin_state
        .catalog()
        .iter()
        .cloned()
        .map(to_protocol_plugin)
        .collect(),
    },
  )
}
