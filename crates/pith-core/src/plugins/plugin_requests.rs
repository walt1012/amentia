use std::path::PathBuf;

use pith_plugin_host::{inspect_plugin_bundle, install_plugin_bundle, remove_local_plugin_bundle};
use pith_protocol::{
  JsonRpcRequest, JsonRpcResponse, PluginInstallParams, PluginInstallResult, PluginListResult,
  PluginRemoveParams, PluginRemoveResult, PluginSetEnabledParams, PluginSetEnabledResult,
};

use crate::protocol_adapters::{
  build_protocol_capability_registry, build_protocol_command_registry,
  build_protocol_connector_registry, build_protocol_hook_registry, to_protocol_plugin,
};
use crate::request_params::parse_required_params;
use crate::runtime_plugins::PluginEnableError;
use crate::RuntimeContext;

pub(crate) fn handle_plugin_capability_registry(
  context: &RuntimeContext,
  request: JsonRpcRequest,
) -> JsonRpcResponse {
  JsonRpcResponse::success(
    request.id,
    &build_protocol_capability_registry(context.plugin_state.catalog()),
  )
}

pub(crate) fn handle_plugin_command_registry(
  context: &RuntimeContext,
  request: JsonRpcRequest,
) -> JsonRpcResponse {
  JsonRpcResponse::success(
    request.id,
    &build_protocol_command_registry(context.plugin_state.catalog()),
  )
}

pub(crate) fn handle_plugin_connector_registry(
  context: &RuntimeContext,
  request: JsonRpcRequest,
) -> JsonRpcResponse {
  JsonRpcResponse::success(
    request.id,
    &build_protocol_connector_registry(context.plugin_state.catalog()),
  )
}

pub(crate) fn handle_plugin_hook_registry(
  context: &RuntimeContext,
  request: JsonRpcRequest,
) -> JsonRpcResponse {
  JsonRpcResponse::success(
    request.id,
    &build_protocol_hook_registry(context.plugin_state.catalog()),
  )
}

pub(crate) fn handle_plugin_list(
  context: &RuntimeContext,
  request: JsonRpcRequest,
) -> JsonRpcResponse {
  JsonRpcResponse::success(
    request.id,
    &PluginListResult {
      plugins: context
        .plugin_state
        .catalog()
        .iter()
        .cloned()
        .map(to_protocol_plugin)
        .collect(),
    },
  )
}

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
