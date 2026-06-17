use amentia_protocol::{JsonRpcRequest, JsonRpcResponse};

use crate::protocol_adapters::{
  to_protocol_model_bootstrap, to_protocol_model_health, to_protocol_model_probe,
};
use crate::RuntimeContext;

pub(crate) fn handle_model_bootstrap(
  context: &mut RuntimeContext,
  request: JsonRpcRequest,
) -> JsonRpcResponse {
  match context.model_state.bootstrap_pack_metadata() {
    Ok(result) => {
      context.model_state.reset_default();
      JsonRpcResponse::success(request.id, &to_protocol_model_bootstrap(result))
    }
    Err(error) => JsonRpcResponse::error(request.id, -32042, error.to_string()),
  }
}

pub(crate) fn handle_model_health(
  context: &RuntimeContext,
  request: JsonRpcRequest,
) -> JsonRpcResponse {
  JsonRpcResponse::success(
    request.id,
    &to_protocol_model_health(context.model_state.health()),
  )
}

pub(crate) fn handle_model_probe(
  context: &RuntimeContext,
  request: JsonRpcRequest,
) -> JsonRpcResponse {
  JsonRpcResponse::success(
    request.id,
    &to_protocol_model_probe(context.model_state.probe()),
  )
}
