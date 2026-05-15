use pith_protocol::JsonRpcResponse;
use serde_json::Value;

pub(crate) struct PluginLifecycleRecovery<'a> {
  pub(crate) operation: &'a str,
  pub(crate) status: &'a str,
  pub(crate) plugin_id: Option<&'a str>,
  pub(crate) source_path: Option<&'a str>,
  pub(crate) repair_hint: Option<&'a str>,
}

pub(crate) fn plugin_lifecycle_error_response(
  request_id: Value,
  code: i32,
  message: impl Into<String>,
  recovery: PluginLifecycleRecovery<'_>,
) -> JsonRpcResponse {
  let blocker = message.into();
  let mut data = serde_json::json!({
    "pluginLifecycleOperation": recovery.operation,
    "pluginLifecycleStatus": recovery.status,
    "lifecycleBlocker": blocker.clone(),
  });
  if let Some(plugin_id) = recovery.plugin_id.filter(|value| !value.is_empty()) {
    data["pluginId"] = serde_json::json!(plugin_id);
  }
  if let Some(source_path) = recovery.source_path.filter(|value| !value.is_empty()) {
    data["sourcePath"] = serde_json::json!(source_path);
  }
  if let Some(repair_hint) = recovery.repair_hint.filter(|value| !value.is_empty()) {
    data["lifecycleRepairHint"] = serde_json::json!(repair_hint);
  }
  JsonRpcResponse::error_with_data(request_id, code, blocker, &data)
}
