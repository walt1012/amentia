use pith_protocol::{JsonRpcRequest, JsonRpcResponse, PluginRefreshResult};

use crate::protocol_adapters::to_protocol_plugin;
use crate::RuntimeContext;

pub(crate) fn handle_plugin_refresh(
  context: &mut RuntimeContext,
  request: JsonRpcRequest,
) -> JsonRpcResponse {
  match context.refresh_plugins_with_runtime_state_fallback() {
    Ok(state_warning) => JsonRpcResponse::success(
      request.id,
      &PluginRefreshResult {
        plugins: context
          .plugin_state
          .catalog()
          .iter()
          .cloned()
          .map(to_protocol_plugin)
          .collect(),
        state_warning: state_warning.map(|error| error.to_string()),
      },
    ),
    Err(error) => JsonRpcResponse::error_with_data(
      request.id,
      -32055,
      error.to_string(),
      &serde_json::json!({
        "pluginRefreshStatus": "failed",
        "pluginRefreshRepairHint": "Check plugin root permissions and refresh plugins again.",
      }),
    ),
  }
}
