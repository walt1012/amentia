use std::path::PathBuf;

use pith_plugin_host::remove_local_plugin_bundle;
use pith_protocol::{JsonRpcRequest, JsonRpcResponse, PluginRemoveParams, PluginRemoveResult};

use super::plugin_lifecycle_recovery::{plugin_lifecycle_error_response, PluginLifecycleRecovery};
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
      Err(error) => {
        return plugin_remove_error_response(
          request.id,
          -32054,
          "removeFailed",
          None,
          &params.manifest_path,
          error.to_string(),
          Some("Choose a local plugin installed under the configured plugin root."),
        );
      }
    };

  let mut cleanup_error = None;
  if let Err(error) =
    context.delete_plugin_connector_credentials_for_plugin(&removed_plugin.plugin_id)
  {
    cleanup_error = Some(error);
  }
  if let Err(error) = context.delete_plugin_state(&removed_plugin.plugin_id) {
    if cleanup_error.is_none() {
      cleanup_error = Some(error);
    }
  }
  context
    .plugin_state
    .clear_connector_credentials_for_plugin(&removed_plugin.plugin_id);
  match context.refresh_plugins_with_runtime_state_fallback() {
    Ok(Some(error)) => {
      if cleanup_error.is_none() {
        cleanup_error = Some(error);
      }
    }
    Ok(None) => {}
    Err(error) => {
      return plugin_remove_error_response(
        request.id,
        -32010,
        "refreshFailed",
        Some(&removed_plugin.plugin_id),
        &params.manifest_path,
        error.to_string(),
        Some("Refresh plugin state, then retry removal if the plugin still appears."),
      );
    }
  }
  if let Some(error) = cleanup_error {
    return plugin_remove_error_response(
      request.id,
      -32010,
      "cleanupFailed",
      Some(&removed_plugin.plugin_id),
      &params.manifest_path,
      error.to_string(),
      Some("Check local storage permissions and refresh plugin state."),
    );
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

fn plugin_remove_error_response(
  request_id: serde_json::Value,
  code: i32,
  status: &str,
  plugin_id: Option<&str>,
  source_path: &str,
  message: impl Into<String>,
  repair_hint: Option<&str>,
) -> JsonRpcResponse {
  plugin_lifecycle_error_response(
    request_id,
    code,
    message,
    PluginLifecycleRecovery {
      operation: "remove",
      status,
      plugin_id,
      source_path: Some(source_path),
      repair_hint,
    },
  )
}
