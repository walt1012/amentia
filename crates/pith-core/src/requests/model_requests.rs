use pith_model_runtime::LocalModelRuntime;
use pith_protocol::{JsonRpcRequest, JsonRpcResponse};

use crate::protocol_adapters::{to_protocol_model_bootstrap, to_protocol_model_health};
use crate::RuntimeContext;

pub(crate) fn handle_model_bootstrap(
  context: &mut RuntimeContext,
  request: JsonRpcRequest,
) -> JsonRpcResponse {
  match context.model_runtime.bootstrap_pack_metadata() {
    Ok(result) => {
      context.model_runtime = LocalModelRuntime::new_default();
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
    &to_protocol_model_health(context.model_runtime.health()),
  )
}
