use pith_protocol::{
  JsonRpcRequest, JsonRpcResponse, PluginSetEnabledParams, PluginSetEnabledResult,
};

use crate::protocol_adapters::to_protocol_plugin;
use crate::request_params::parse_required_params;
use crate::runtime_plugins::PluginEnableError;
use crate::RuntimeContext;

pub(crate) fn handle_plugin_set_enabled(
  context: &mut RuntimeContext,
  request: JsonRpcRequest,
) -> JsonRpcResponse {
  let params = match parse_required_params::<PluginSetEnabledParams>(&request, "plugin/setEnabled")
  {
    Ok(params) => params,
    Err(response) => return response,
  };

  let updated_plugin = match context
    .plugin_state
    .set_enabled(&params.plugin_id, params.enabled)
  {
    Ok(plugin) => plugin,
    Err(PluginEnableError::NotFound) => {
      return JsonRpcResponse::error(request.id, -32050, "Plugin not found");
    }
    Err(PluginEnableError::InvalidManifest(message)) => {
      return JsonRpcResponse::error(request.id, -32051, message);
    }
  };
  let plugin_id = updated_plugin.id.clone();
  let plugin_enabled = updated_plugin.enabled;

  if let Err(error) = context.persist_plugin_enabled(&plugin_id, plugin_enabled) {
    return JsonRpcResponse::error(request.id, -32010, error.to_string());
  }

  JsonRpcResponse::success(
    request.id,
    &PluginSetEnabledResult {
      plugin: to_protocol_plugin(updated_plugin),
    },
  )
}
