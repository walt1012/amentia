use std::time::{SystemTime, UNIX_EPOCH};

use pith_plugin_host::PluginConnectorEntry;
use pith_protocol::{
  JsonRpcRequest, JsonRpcResponse, PluginConnectorCredentialParams,
  PluginConnectorCredentialResult, PluginConnectorSummary,
};

use crate::protocol_adapters::build_protocol_connector_registry;
use crate::request_params::parse_required_params;
use crate::runtime_plugins::PluginConnectorCredentialState;
use crate::RuntimeContext;

pub(crate) fn handle_plugin_connector_authorize(
  context: &mut RuntimeContext,
  request: JsonRpcRequest,
) -> JsonRpcResponse {
  let params = match parse_required_params::<PluginConnectorCredentialParams>(
    &request,
    "plugin/connectorAuthorize",
  ) {
    Ok(params) => params,
    Err(response) => return response,
  };

  let connector = match find_connector(context, &params.connector_id) {
    Some(connector) => connector,
    None => return JsonRpcResponse::error(request.id, -32055, "Plugin connector not found"),
  };
  if !connector.enabled {
    return JsonRpcResponse::error(request.id, -32056, "Plugin connector is disabled");
  }
  if !connector.auth_required {
    return JsonRpcResponse::error(
      request.id,
      -32057,
      "Plugin connector does not require credentials",
    );
  }

  let timestamp = match current_unix_timestamp() {
    Ok(timestamp) => timestamp,
    Err(message) => return JsonRpcResponse::error(request.id, -32010, message),
  };
  let credential = PluginConnectorCredentialState {
    connector_id: connector.connector_id.clone(),
    plugin_id: connector.plugin_id.clone(),
    credential_store: connector
      .credential_store
      .clone()
      .unwrap_or_else(|| "local".to_string()),
    credential_label: format!("{} authorization marker", connector.display_name),
    authorized_at: timestamp,
    updated_at: timestamp,
  };

  if let Err(error) = context.persist_plugin_connector_credential(&credential) {
    return JsonRpcResponse::error(request.id, -32010, error.to_string());
  }
  context.plugin_state.set_connector_credential(credential);

  connector_success(context, request.id, &params.connector_id)
}

pub(crate) fn handle_plugin_connector_clear_credential(
  context: &mut RuntimeContext,
  request: JsonRpcRequest,
) -> JsonRpcResponse {
  let params = match parse_required_params::<PluginConnectorCredentialParams>(
    &request,
    "plugin/connectorClearCredential",
  ) {
    Ok(params) => params,
    Err(response) => return response,
  };

  if find_connector(context, &params.connector_id).is_none() {
    return JsonRpcResponse::error(request.id, -32055, "Plugin connector not found");
  }
  if let Err(error) = context.delete_plugin_connector_credential(&params.connector_id) {
    return JsonRpcResponse::error(request.id, -32010, error.to_string());
  }
  context
    .plugin_state
    .clear_connector_credential(&params.connector_id);

  connector_success(context, request.id, &params.connector_id)
}

fn connector_success(
  context: &RuntimeContext,
  request_id: serde_json::Value,
  connector_id: &str,
) -> JsonRpcResponse {
  match find_protocol_connector(context, connector_id) {
    Some(connector) => {
      JsonRpcResponse::success(request_id, &PluginConnectorCredentialResult { connector })
    }
    None => JsonRpcResponse::error(request_id, -32055, "Plugin connector not found"),
  }
}

fn find_connector(context: &RuntimeContext, connector_id: &str) -> Option<PluginConnectorEntry> {
  context
    .plugin_state
    .connector_entries()
    .into_iter()
    .find(|connector| connector.connector_id == connector_id)
}

fn find_protocol_connector(
  context: &RuntimeContext,
  connector_id: &str,
) -> Option<PluginConnectorSummary> {
  build_protocol_connector_registry(&context.plugin_state)
    .connectors
    .into_iter()
    .find(|connector| connector.connector_id == connector_id)
}

fn current_unix_timestamp() -> Result<i64, String> {
  SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .map(|duration| duration.as_secs() as i64)
    .map_err(|error| format!("System clock is before Unix epoch: {error}"))
}
