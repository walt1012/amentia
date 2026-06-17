use amentia_protocol::{
  HealthPingResult, InitializeParams, InitializeResult, JsonRpcRequest, JsonRpcResponse,
  ServerCapabilities, ServerInfo,
};

use crate::request_params::parse_required_params;
use crate::RuntimeContext;

pub(crate) fn handle_initialize(
  context: &RuntimeContext,
  request: JsonRpcRequest,
) -> JsonRpcResponse {
  let params = match parse_required_params::<InitializeParams>(&request, "initialize") {
    Ok(params) => params,
    Err(response) => return response,
  };

  let _client = params.client_info;

  JsonRpcResponse::success(
    request.id,
    &InitializeResult {
      server_info: ServerInfo {
        name: context.identity.server_name.clone(),
        version: context.identity.server_version.clone(),
      },
      protocol_version: "0.1.0".to_string(),
      capabilities: ServerCapabilities {
        supports_memory: true,
        supports_threads: true,
        supports_tools: true,
        supports_plugins: context.plugin_state.catalog_len() > 0,
        supports_runtime_readiness: true,
      },
    },
  )
}

pub(crate) fn handle_health_ping(request: JsonRpcRequest) -> JsonRpcResponse {
  JsonRpcResponse::success(
    request.id,
    &HealthPingResult {
      status: "ok".to_string(),
    },
  )
}
