use cavell_protocol::{
  methods, HealthPingResult, InitializeParams, InitializeResult, JsonRpcRequest, JsonRpcResponse,
  ServerCapabilities, ServerInfo, ThreadListResult,
};

#[derive(Debug, Clone)]
pub struct RuntimeContext {
  server_name: String,
  server_version: String,
}

impl RuntimeContext {
  pub fn new() -> Self {
    Self {
      server_name: "cavell-runtime".to_string(),
      server_version: env!("CARGO_PKG_VERSION").to_string(),
    }
  }
}

pub fn handle_request(context: &RuntimeContext, request: JsonRpcRequest) -> JsonRpcResponse {
  match request.method.as_str() {
    methods::INITIALIZE => handle_initialize(context, request),
    methods::HEALTH_PING => JsonRpcResponse::success(
      request.id,
      &HealthPingResult {
        status: "ok".to_string(),
      },
    ),
    methods::THREAD_LIST => JsonRpcResponse::success(
      request.id,
      &ThreadListResult { threads: vec![] },
    ),
    _ => JsonRpcResponse::error(request.id, -32601, "Method not found"),
  }
}

fn handle_initialize(context: &RuntimeContext, request: JsonRpcRequest) -> JsonRpcResponse {
  let params = match request.params {
    Some(value) => match serde_json::from_value::<InitializeParams>(value) {
      Ok(params) => params,
      Err(error) => {
        return JsonRpcResponse::error(
          request.id,
          -32602,
          format!("Invalid initialize params: {error}"),
        )
      }
    },
    None => {
      return JsonRpcResponse::error(request.id, -32602, "Missing initialize params");
    }
  };

  let _client = params.client_info;

  JsonRpcResponse::success(
    request.id,
    &InitializeResult {
      server_info: ServerInfo {
        name: context.server_name.clone(),
        version: context.server_version.clone(),
      },
      protocol_version: "0.1.0".to_string(),
      capabilities: ServerCapabilities {
        supports_threads: true,
        supports_tools: false,
        supports_plugins: false,
      },
    },
  )
}

#[cfg(test)]
mod tests {
  use super::*;
  use serde_json::{json, Value};

  fn request(method: &str, params: Option<Value>) -> JsonRpcRequest {
    JsonRpcRequest {
      id: json!(1),
      method: method.to_string(),
      params,
    }
  }

  #[test]
  fn initialize_request_returns_capabilities() {
    let context = RuntimeContext::new();
    let response = handle_request(
      &context,
      request(
        methods::INITIALIZE,
        Some(json!({
          "clientInfo": {
            "name": "cavell-tests",
            "version": "0.1.0"
          }
        })),
      ),
    );

    assert!(response.error.is_none());
    let result = response.result.expect("initialize result");
    assert_eq!(result["protocolVersion"], "0.1.0");
    assert_eq!(result["capabilities"]["supportsThreads"], true);
  }

  #[test]
  fn health_ping_returns_ok() {
    let context = RuntimeContext::new();
    let response = handle_request(&context, request(methods::HEALTH_PING, None));

    assert!(response.error.is_none());
    let result = response.result.expect("health result");
    assert_eq!(result["status"], "ok");
  }

  #[test]
  fn unknown_method_returns_json_rpc_error() {
    let context = RuntimeContext::new();
    let response = handle_request(&context, request("unknown/method", None));

    assert!(response.result.is_none());
    let error = response.error.expect("error payload");
    assert_eq!(error.code, -32601);
  }
}
