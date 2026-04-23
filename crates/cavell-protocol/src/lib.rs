use serde::{Deserialize, Serialize};
use serde_json::Value;

pub mod methods {
  pub const INITIALIZE: &str = "initialize";
  pub const HEALTH_PING: &str = "health/ping";
  pub const THREAD_START: &str = "thread/start";
  pub const THREAD_LIST: &str = "thread/list";
  pub const TURN_START: &str = "turn/start";
}

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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientInfo {
  pub name: String,
  pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializeParams {
  pub client_info: ClientInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerInfo {
  pub name: String,
  pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerCapabilities {
  pub supports_threads: bool,
  pub supports_tools: bool,
  pub supports_plugins: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializeResult {
  pub server_info: ServerInfo,
  pub protocol_version: String,
  pub capabilities: ServerCapabilities,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthPingResult {
  pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadSummary {
  pub id: String,
  pub title: String,
  pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadListResult {
  pub threads: Vec<ThreadSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadStartParams {
  pub title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadStartResult {
  pub thread: ThreadSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TurnStartParams {
  pub thread_id: String,
  pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineItem {
  pub kind: String,
  pub title: String,
  pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TurnStartResult {
  pub turn_id: String,
  pub thread_id: String,
  pub items: Vec<TimelineItem>,
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
      }),
    }
  }
}
