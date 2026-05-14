use std::path::PathBuf;

use pith_plugin_host::PluginCatalogEntry;
use pith_plugin_host::{inspect_plugin_bundle, install_plugin_bundle};
use pith_protocol::{
  JsonRpcRequest, JsonRpcResponse, PluginInspectParams, PluginInspectResult, PluginInstallParams,
  PluginInstallResult,
};

use crate::protocol_adapters::to_protocol_plugin;
use crate::request_params::parse_required_params;
use crate::RuntimeContext;

pub(crate) fn handle_plugin_inspect(
  context: &mut RuntimeContext,
  request: JsonRpcRequest,
) -> JsonRpcResponse {
  let params = match parse_required_params::<PluginInspectParams>(&request, "plugin/inspect") {
    Ok(params) => params,
    Err(response) => return response,
  };

  let source_path = PathBuf::from(&params.source_path);
  match inspect_plugin_bundle(&source_path) {
    Ok(plugin) => {
      let install_readiness = plugin_install_readiness(context, &plugin);
      JsonRpcResponse::success(
        request.id,
        &PluginInspectResult {
          plugin: to_protocol_plugin(plugin),
          install_status: install_readiness.status,
          install_blocker: install_readiness.blocker,
          install_repair_hint: install_readiness.repair_hint,
        },
      )
    }
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
  let install_readiness = plugin_install_readiness(context, &candidate_plugin);
  if let Some(blocker) = install_readiness.blocker {
    return JsonRpcResponse::error(
      request.id,
      -32053,
      blocker,
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

struct PluginInstallReadiness {
  status: String,
  blocker: Option<String>,
  repair_hint: Option<String>,
}

fn plugin_install_readiness(
  context: &RuntimeContext,
  candidate_plugin: &PluginCatalogEntry,
) -> PluginInstallReadiness {
  if context
    .plugin_state
    .contains_plugin_id(&candidate_plugin.id)
  {
    return PluginInstallReadiness {
      status: "alreadyInstalled".to_string(),
      blocker: Some(format!(
        "Plugin `{}` is already installed",
        candidate_plugin.display_name
      )),
      repair_hint: Some(
        "Remove the existing local plugin first, or change the plugin name before installing this copy."
          .to_string(),
      ),
    };
  }

  PluginInstallReadiness {
    status: "ready".to_string(),
    blocker: None,
    repair_hint: None,
  }
}
