use std::collections::HashMap;
use std::path::PathBuf;

use pith_plugin_host::PluginCatalogEntry;
use pith_protocol::{JsonRpcRequest, JsonRpcResponse, PluginRefreshResult};
use pith_storage::RuntimeStore;

use crate::plugin_catalog_state::{apply_plugin_states, load_plugin_catalog};
use crate::protocol_adapters::to_protocol_plugin;
use crate::RuntimeContext;

#[derive(Debug)]
pub struct PreparedPluginRefresh {
  request_id: serde_json::Value,
  roots: Vec<PathBuf>,
  store: Option<RuntimeStore>,
  runtime_states: HashMap<String, bool>,
}

#[derive(Debug)]
pub struct CompletedPluginRefresh {
  request_id: serde_json::Value,
  output: std::result::Result<PluginRefreshOutput, (i32, String)>,
}

#[derive(Debug)]
struct PluginRefreshOutput {
  plugins: Vec<PluginCatalogEntry>,
  state_warning: Option<String>,
}

pub(crate) fn handle_plugin_refresh(
  context: &mut RuntimeContext,
  request: JsonRpcRequest,
) -> JsonRpcResponse {
  let prepared = match prepare_plugin_refresh(context, request) {
    Ok(prepared) => prepared,
    Err(response) => return response,
  };
  let completed = execute_prepared_plugin_refresh(prepared);
  complete_prepared_plugin_refresh(context, completed)
}

pub fn prepare_plugin_refresh(
  context: &mut RuntimeContext,
  request: JsonRpcRequest,
) -> std::result::Result<PreparedPluginRefresh, JsonRpcResponse> {
  let runtime_states = context
    .plugin_state
    .catalog()
    .iter()
    .map(|plugin| (plugin.id.clone(), plugin.enabled))
    .collect::<HashMap<_, _>>();
  Ok(PreparedPluginRefresh {
    request_id: request.id,
    roots: context.plugin_state.roots().to_vec(),
    store: context.persistence_state.store().cloned(),
    runtime_states,
  })
}

pub fn execute_prepared_plugin_refresh(prepared: PreparedPluginRefresh) -> CompletedPluginRefresh {
  let PreparedPluginRefresh {
    request_id,
    roots,
    store,
    runtime_states,
  } = prepared;
  let (plugin_states, state_warning) = match load_persisted_plugin_states(store.as_ref()) {
    Ok(plugin_states) => (plugin_states, None),
    Err(error) => (runtime_states, Some(error)),
  };
  let output = load_plugin_catalog(&roots)
    .map(|plugins| PluginRefreshOutput {
      plugins: apply_plugin_states(plugins, &plugin_states),
      state_warning,
    })
    .map_err(|error| (-32055, error.to_string()));

  CompletedPluginRefresh { request_id, output }
}

fn load_persisted_plugin_states(
  store: Option<&RuntimeStore>,
) -> std::result::Result<HashMap<String, bool>, String> {
  match store {
    Some(store) => store
      .load_plugin_states()
      .map_err(|error| error.to_string()),
    None => Ok(HashMap::new()),
  }
}

pub fn complete_prepared_plugin_refresh(
  context: &mut RuntimeContext,
  completed: CompletedPluginRefresh,
) -> JsonRpcResponse {
  match completed.output {
    Ok(output) => {
      context.plugin_state.replace_catalog(output.plugins.clone());
      JsonRpcResponse::success(
        completed.request_id,
        &PluginRefreshResult {
          plugins: output.plugins.into_iter().map(to_protocol_plugin).collect(),
          state_warning: output.state_warning,
        },
      )
    }
    Err((code, message)) => plugin_refresh_error_response(completed.request_id, code, message),
  }
}

fn plugin_refresh_error_response(
  request_id: serde_json::Value,
  code: i32,
  message: String,
) -> JsonRpcResponse {
  JsonRpcResponse::error_with_data(
    request_id,
    code,
    message,
    &serde_json::json!({
      "pluginRefreshStatus": "failed",
      "pluginRefreshRepairHint": "Check plugin root permissions and refresh plugins again.",
    }),
  )
}
