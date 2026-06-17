use amentia_protocol::{JsonRpcRequest, JsonRpcResponse};
use serde::de::DeserializeOwned;

pub(crate) fn parse_required_params<T>(
  request: &JsonRpcRequest,
  method_label: &str,
) -> Result<T, JsonRpcResponse>
where
  T: DeserializeOwned,
{
  let Some(value) = request.params.clone() else {
    return Err(JsonRpcResponse::error(
      request.id.clone(),
      -32602,
      format!("Missing {method_label} params"),
    ));
  };

  serde_json::from_value::<T>(value).map_err(|error| {
    JsonRpcResponse::error(
      request.id.clone(),
      -32602,
      format!("Invalid {method_label} params: {error}"),
    )
  })
}
