use pith_plugin_host::channel_adapter_blocker_for_manifest;
use pith_protocol::{
  JsonRpcRequest, JsonRpcResponse, PluginSetEnabledParams, PluginSetEnabledResult,
};

use super::plugin_lifecycle_recovery::{plugin_lifecycle_error_response, PluginLifecycleRecovery};
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

  let operation = if params.enabled { "enable" } else { "disable" };
  let Some(plugin) = context.plugin_state.find(&params.plugin_id) else {
    return plugin_enable_error_response(
      request.id,
      -32050,
      operation,
      "notFound",
      &params.plugin_id,
      "Plugin not found",
      Some("Refresh the plugin list and select an installed plugin."),
    );
  };
  if plugin.status != "ready" {
    let message = plugin
      .validation_error
      .clone()
      .unwrap_or_else(|| "Plugin manifest is invalid".to_string());
    let repair_hint = plugin.validation_hint.clone().unwrap_or_else(|| {
      "Fix the plugin manifest, then refresh or reinstall the plugin.".to_string()
    });
    return plugin_enable_error_response(
      request.id,
      -32051,
      operation,
      "invalidManifest",
      &params.plugin_id,
      message,
      Some(repair_hint.as_str()),
    );
  }

  if params.enabled && plugin_has_channel(&plugin.capabilities) {
    match channel_adapter_blocker_for_manifest(std::path::Path::new(&plugin.manifest_path)) {
      Ok(Some(blocker)) => {
        let message = format!(
          "Plugin channel `{}` uses protocol `{}`, but Pith has not shipped that channel adapter yet.",
          blocker.display_name, blocker.protocol
        );
        return plugin_enable_error_response(
          request.id,
          -32058,
          operation,
          "channelAdapterPending",
          &params.plugin_id,
          message,
          Some("Keep this plugin disabled until the channel adapter is implemented."),
        );
      }
      Ok(None) => {}
      Err(error) => {
        return plugin_enable_error_response(
          request.id,
          -32051,
          operation,
          "invalidManifest",
          &params.plugin_id,
          error.to_string(),
          Some("Fix the plugin manifest, then refresh or reinstall the plugin."),
        );
      }
    }
  }

  if let Err(error) = context.persist_plugin_enabled(&params.plugin_id, params.enabled) {
    return plugin_enable_error_response(
      request.id,
      -32010,
      operation,
      "persistFailed",
      &params.plugin_id,
      error.to_string(),
      Some("Check local storage permissions and try again."),
    );
  }

  let updated_plugin = match context
    .plugin_state
    .set_enabled(&params.plugin_id, params.enabled)
  {
    Ok(plugin) => plugin,
    Err(PluginEnableError::NotFound) => {
      return plugin_enable_error_response(
        request.id,
        -32050,
        operation,
        "notFound",
        &params.plugin_id,
        "Plugin not found",
        Some("Refresh the plugin list and select an installed plugin."),
      );
    }
    Err(PluginEnableError::InvalidManifest(message)) => {
      return plugin_enable_error_response(
        request.id,
        -32051,
        operation,
        "invalidManifest",
        &params.plugin_id,
        message,
        Some("Fix the plugin manifest, then refresh or reinstall the plugin."),
      );
    }
  };

  JsonRpcResponse::success(
    request.id,
    &PluginSetEnabledResult {
      plugin: to_protocol_plugin(updated_plugin),
    },
  )
}

fn plugin_has_channel(capabilities: &[String]) -> bool {
  capabilities
    .iter()
    .any(|capability| capability.starts_with("channel:"))
}

fn plugin_enable_error_response(
  request_id: serde_json::Value,
  code: i32,
  operation: &str,
  status: &str,
  plugin_id: &str,
  message: impl Into<String>,
  repair_hint: Option<&str>,
) -> JsonRpcResponse {
  plugin_lifecycle_error_response(
    request_id,
    code,
    message,
    PluginLifecycleRecovery {
      operation,
      status,
      plugin_id: Some(plugin_id),
      source_path: None,
      repair_hint,
    },
  )
}
