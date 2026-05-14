use std::path::PathBuf;

use pith_plugin_host::{inspect_plugin_bundle, install_plugin_bundle};
use pith_protocol::{
  JsonRpcRequest, JsonRpcResponse, PluginInspectParams, PluginInspectResult,
  PluginInstallParams, PluginInstallResult,
};

use crate::protocol_adapters::to_protocol_plugin;
use crate::request_params::parse_required_params;
use crate::RuntimeContext;

pub(crate) fn handle_plugin_inspect(
  _context: &mut RuntimeContext,
  request: JsonRpcRequest,
) -> JsonRpcResponse {
  let params = match parse_required_params::<PluginInspectParams>(&request, "plugin/inspect") {
    Ok(params) => params,
    Err(response) => return response,
  };

  let source_path = PathBuf::from(&params.source_path);
  match inspect_plugin_bundle(&source_path) {
    Ok(plugin) => JsonRpcResponse::success(
      request.id,
      &PluginInspectResult {
        plugin: to_protocol_plugin(plugin),
      },
    ),
    Err(error) => JsonRpcResponse::error(request.id, -32053, error.to_string()),
  }
}

pub(crate) fn handle_plugin_install(
  context: &mut RuntimeContext,
  request: JsonRpcRequest,
) -> JsonRpcResponse {
  let params = match parse_required_params::<PluginInstallParams>(&request, "plugin/install") {
    Ok(params) => params,
    Err(response) => return response,
  };

  let source_path = PathBuf::from(&params.source_path);
  let candidate_plugin = match inspect_plugin_bundle(&source_path) {
    Ok(plugin) => plugin,
    Err(error) => return JsonRpcResponse::error(request.id, -32053, error.to_string()),
  };
  if context
    .plugin_state
    .contains_plugin_id(&candidate_plugin.id)
  {
    return JsonRpcResponse::error(
      request.id,
      -32053,
      format!(
        "Plugin `{}` is already installed",
        candidate_plugin.display_name
      ),
    );
  }
  let installed_plugin =
    match install_plugin_bundle(&source_path, context.plugin_state.install_root()) {
      Ok(plugin) => plugin,
      Err(error) => return JsonRpcResponse::error(request.id, -32053, error.to_string()),
    };

  if let Err(error) = context.refresh_plugins() {
    return JsonRpcResponse::error(request.id, -32010, error.to_string());
  }

  let refreshed_plugin = context
    .plugin_state
    .find(&installed_plugin.id)
    .cloned()
    .unwrap_or(installed_plugin);

  JsonRpcResponse::success(
    request.id,
    &PluginInstallResult {
      plugin: to_protocol_plugin(refreshed_plugin),
    },
  )
}
