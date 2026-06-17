use std::time::{SystemTime, UNIX_EPOCH};

use amentia_plugin_host::PluginConnectorEntry;
use amentia_protocol::{
  JsonRpcRequest, JsonRpcResponse, PluginConnectorCredentialParams,
  PluginConnectorCredentialResult, PluginConnectorSummary,
};
use serde_json::{json, Value};

use crate::protocol_adapters::build_protocol_connector_registry;
use crate::request_params::parse_required_params;
use crate::runtime_plugins::PluginConnectorCredentialState;
use crate::secure_credentials;
use crate::RuntimeContext;

const MISSING_CONNECTOR_REPAIR_HINT: &str =
  "Refresh plugins, reinstall the plugin, or choose an available connection from the plugin panel.";
const AUTHORIZE_DISABLED_CONNECTOR_REPAIR_HINT: &str =
  "Enable the plugin before authorizing this connection.";
const NOT_REQUIRED_CONNECTOR_REPAIR_HINT: &str =
  "Run the action without connection authorization; this connection does not require credentials.";
const MISSING_SECRET_REPAIR_HINT: &str =
  "Paste the connection token or API key, then retry authorization.";
const AUTHORIZE_STORE_REPAIR_HINT: &str =
  "Check local storage permissions, then retry connection authorization.";
const CLEAR_STORE_REPAIR_HINT: &str =
  "Check local storage permissions, then retry clearing the connection credential.";

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
    None => {
      return connector_error_response(
        request.id,
        -32055,
        &params.connector_id,
        None,
        "Plugin connector not found",
        "missingConnector",
        MISSING_CONNECTOR_REPAIR_HINT,
      );
    }
  };
  if !connector.enabled {
    return connector_error_response(
      request.id,
      -32056,
      &params.connector_id,
      Some(&connector.plugin_id),
      "Plugin connector is disabled",
      "disabled",
      AUTHORIZE_DISABLED_CONNECTOR_REPAIR_HINT,
    );
  }
  if !connector.auth_required {
    return connector_error_response(
      request.id,
      -32057,
      &params.connector_id,
      Some(&connector.plugin_id),
      "Plugin connector does not require credentials",
      "notRequired",
      NOT_REQUIRED_CONNECTOR_REPAIR_HINT,
    );
  }

  let timestamp = match current_unix_timestamp() {
    Ok(timestamp) => timestamp,
    Err(message) => {
      return connector_error_response(
        request.id,
        -32010,
        &params.connector_id,
        Some(&connector.plugin_id),
        message,
        "clockError",
        "Check the system clock, then retry connector authorization.",
      );
    }
  };
  let credential_secret = normalized_credential_secret(params.credential_secret.as_deref());
  if credential_secret.is_none() && connector_requires_secret(&connector) {
    return connector_error_response(
      request.id,
      -32058,
      &params.connector_id,
      Some(&connector.plugin_id),
      "Plugin connector requires a token or API key",
      "missingCredentialSecret",
      MISSING_SECRET_REPAIR_HINT,
    );
  }
  if let Some(secret) = credential_secret.as_deref() {
    if let Err(error) = secure_credentials::save_connector_secret(&params.connector_id, secret) {
      return connector_error_response(
        request.id,
        -32010,
        &params.connector_id,
        Some(&connector.plugin_id),
        error.to_string(),
        "credentialStoreError",
        AUTHORIZE_STORE_REPAIR_HINT,
      );
    }
  }
  let credential = PluginConnectorCredentialState {
    connector_id: connector.connector_id.clone(),
    plugin_id: connector.plugin_id.clone(),
    credential_store: connector.credential_store.clone().unwrap_or_else(|| {
      if cfg!(target_os = "macos") {
        "macOS Keychain".to_string()
      } else {
        "runtime session".to_string()
      }
    }),
    credential_label: params
      .credential_label
      .clone()
      .unwrap_or_else(|| format!("{} authorization marker", connector.display_name)),
    credential_secret,
    authorized_at: timestamp,
    updated_at: timestamp,
  };

  if let Err(error) = context.persist_plugin_connector_credential(&credential) {
    let _ = secure_credentials::delete_connector_secret(&params.connector_id);
    return connector_error_response(
      request.id,
      -32010,
      &params.connector_id,
      Some(&connector.plugin_id),
      error.to_string(),
      "credentialStoreError",
      AUTHORIZE_STORE_REPAIR_HINT,
    );
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

  let connector = match find_connector(context, &params.connector_id) {
    Some(connector) => connector,
    None => {
      return connector_error_response(
        request.id,
        -32055,
        &params.connector_id,
        None,
        "Plugin connector not found",
        "missingConnector",
        MISSING_CONNECTOR_REPAIR_HINT,
      );
    }
  };
  if let Err(error) = secure_credentials::delete_connector_secret(&params.connector_id) {
    return connector_error_response(
      request.id,
      -32010,
      &params.connector_id,
      Some(&connector.plugin_id),
      error.to_string(),
      "credentialStoreError",
      CLEAR_STORE_REPAIR_HINT,
    );
  }
  context
    .plugin_state
    .clear_connector_credential(&params.connector_id);
  if let Err(error) = context.delete_plugin_connector_credential(&params.connector_id) {
    return connector_error_response(
      request.id,
      -32010,
      &params.connector_id,
      Some(&connector.plugin_id),
      error.to_string(),
      "credentialStoreError",
      CLEAR_STORE_REPAIR_HINT,
    );
  }

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
    None => connector_error_response(
      request_id,
      -32055,
      connector_id,
      None,
      "Plugin connector not found",
      "missingConnector",
      MISSING_CONNECTOR_REPAIR_HINT,
    ),
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

fn connector_error_response(
  request_id: Value,
  code: i32,
  connector_id: &str,
  plugin_id: Option<&str>,
  message: impl Into<String>,
  connector_status: &str,
  connector_repair_hint: &str,
) -> JsonRpcResponse {
  let mut data = json!({
    "connectorId": connector_id,
    "connectorStatus": connector_status,
    "connectorRepairHint": connector_repair_hint,
  });
  if let Some(plugin_id) = plugin_id {
    data["pluginId"] = json!(plugin_id);
  }

  JsonRpcResponse::error_with_data(request_id, code, message, &data)
}

fn current_unix_timestamp() -> Result<i64, String> {
  SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .map(|duration| duration.as_secs() as i64)
    .map_err(|error| format!("System clock is before Unix epoch: {error}"))
}

fn normalized_credential_secret(secret: Option<&str>) -> Option<String> {
  secret
    .map(str::trim)
    .filter(|secret| !secret.is_empty())
    .map(str::to_string)
}

fn connector_requires_secret(connector: &PluginConnectorEntry) -> bool {
  if !connector.auth_required {
    return false;
  }

  let Some(auth_type) = connector.auth_type.as_deref() else {
    return false;
  };
  let normalized = normalized_auth_type(auth_type);
  normalized == "api_key" || normalized == "apikey"
}

fn normalized_auth_type(auth_type: &str) -> String {
  auth_type
    .trim()
    .to_ascii_lowercase()
    .replace(['-', ' '], "_")
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn api_key_spelling_variants_require_secret() {
    for auth_type in ["api_key", "apiKey", "api-key", " API_KEY "] {
      let connector = connector(Some(auth_type), true);
      assert!(connector_requires_secret(&connector));
    }
  }

  #[test]
  fn non_api_key_or_optional_connectors_do_not_require_secret() {
    assert!(!connector_requires_secret(&connector(Some("oauth2"), true)));
    assert!(!connector_requires_secret(&connector(
      Some("api_key"),
      false
    )));
    assert!(!connector_requires_secret(&connector(None, true)));
  }

  fn connector(auth_type: Option<&str>, auth_required: bool) -> PluginConnectorEntry {
    PluginConnectorEntry {
      connector_id: "notion-connector::notion".to_string(),
      display_name: "Notion".to_string(),
      service: "notion".to_string(),
      plugin_id: "notion-connector".to_string(),
      plugin_display_name: "Notion Connector".to_string(),
      enabled: true,
      status: "ready".to_string(),
      permissions: vec!["network.outbound".to_string()],
      manifest_path: "/tmp/notion/amentia-plugin.json".to_string(),
      homepage: None,
      auth_type: auth_type.map(str::to_string),
      auth_required,
      auth_scopes: vec![],
      credential_store: Some("local".to_string()),
      workflows: vec![],
    }
  }
}
