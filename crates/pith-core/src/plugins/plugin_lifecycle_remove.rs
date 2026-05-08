use std::path::PathBuf;

use pith_plugin_host::remove_local_plugin_bundle;
use pith_protocol::{JsonRpcRequest, JsonRpcResponse, PluginRemoveParams, PluginRemoveResult};

use crate::request_params::parse_required_params;
use crate::RuntimeContext;

pub(crate) fn handle_plugin_remove(
  context: &mut RuntimeContext,
  request: JsonRpcRequest,
) -> JsonRpcResponse {
  let params = match parse_required_params::<PluginRemoveParams>(&request, "plugin/remove") {
    Ok(params) => params,
    Err(response) => return response,
  };

  let manifest_path = PathBuf::from(&params.manifest_path);
  let removed_plugin =
    match remove_local_plugin_bundle(&manifest_path, context.plugin_state.install_root()) {
      Ok(plugin) => plugin,
      Err(error) => return JsonRpcResponse::error(request.id, -32054, error.to_string()),
    };

  if let Err(error) = context.delete_plugin_state(&removed_plugin.plugin_id) {
    return JsonRpcResponse::error(request.id, -32010, error.to_string());
  }
  if let Err(error) = context.refresh_plugins() {
    return JsonRpcResponse::error(request.id, -32010, error.to_string());
  }

  JsonRpcResponse::success(
    request.id,
    &PluginRemoveResult {
      plugin_id: removed_plugin.plugin_id,
      display_name: removed_plugin.display_name,
      removed_path: removed_plugin.removed_path,
    },
  )
}
