use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
  pub id: Value,
  pub method: String,
  #[serde(default)]
  pub params: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
  pub id: Value,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub result: Option<Value>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub error: Option<RpcError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcNotification {
  pub method: String,
  #[serde(default)]
  pub params: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcError {
  pub code: i32,
  pub message: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub data: Option<Value>,
}

impl JsonRpcResponse {
  pub fn success<T>(id: Value, result: &T) -> Self
  where
    T: Serialize,
  {
    Self {
      id,
      result: Some(serde_json::to_value(result).expect("serializable result")),
      error: None,
    }
  }

  pub fn error(id: Value, code: i32, message: impl Into<String>) -> Self {
    Self {
      id,
      result: None,
      error: Some(RpcError {
        code,
        message: message.into(),
        data: None,
      }),
    }
  }

  pub fn error_with_data<T>(id: Value, code: i32, message: impl Into<String>, data: &T) -> Self
  where
    T: Serialize,
  {
    Self {
      id,
      result: None,
      error: Some(RpcError {
        code,
        message: message.into(),
        data: Some(serde_json::to_value(data).expect("serializable error data")),
      }),
    }
  }
}
