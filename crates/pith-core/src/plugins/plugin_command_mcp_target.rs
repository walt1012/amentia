use std::fs::File;
use std::io::Read;
use std::path::Path;

use pith_plugin_host::{
  PluginCommandEntry as HostPluginCommandEntry, PluginCommandExecutionEntry, PluginManifest,
  PluginMcpServerManifest,
};
use pith_protocol::WorkspaceSummary;
use serde_json::json;

use super::plugin_command_runner::{PluginRunnerFailure, PluginRunnerRunResult};
use super::plugin_command_runner_setup::unsupported_execution_error;
use super::plugin_command_types::PluginConnectorExecutionRef;

const PLUGIN_MANIFEST_MAX_BYTES: usize = 64 * 1024;

pub(super) const MCP_INITIALIZE_REQUEST_ID: i64 = 1;
pub(super) const MCP_TOOL_CALL_REQUEST_ID: i64 = 2;

pub(super) struct PluginMcpTarget {
  pub(super) server_id: String,
  pub(super) tool_name: String,
}

pub(super) fn insert_mcp_runner_attributes(
  attributes: &mut std::collections::HashMap<String, String>,
  target: &PluginMcpTarget,
  server: &PluginMcpServerManifest,
) {
  attributes.insert("mcpServerId".to_string(), target.server_id.clone());
  attributes.insert("mcpToolName".to_string(), target.tool_name.clone());
  attributes.insert(
    "mcpTransport".to_string(),
    server
      .transport
      .clone()
      .unwrap_or_else(|| "stdio".to_string()),
  );
}

pub(super) fn mcp_target_for_execution(
  command: &HostPluginCommandEntry,
  execution: &PluginCommandExecutionEntry,
) -> PluginRunnerRunResult<PluginMcpTarget> {
  let entrypoint = execution
    .entrypoint
    .as_deref()
    .map(str::trim)
    .filter(|entrypoint| !entrypoint.is_empty())
    .ok_or_else(|| unsupported_execution_error(command))?;
  let Some((server_id, tool_name)) = entrypoint.split_once('.') else {
    return Err(
      PluginRunnerFailure::empty(
        -32053,
        format!(
          "Plugin command `{}` requires an MCP entrypoint like `server.tool`.",
          command.command_id
        ),
      )
      .boxed(),
    );
  };
  let server_id = server_id.trim();
  let tool_name = tool_name.trim();
  if server_id.is_empty() || tool_name.is_empty() {
    return Err(
      PluginRunnerFailure::empty(
        -32053,
        format!(
          "Plugin command `{}` requires a non-empty MCP server and tool name.",
          command.command_id
        ),
      )
      .boxed(),
    );
  }

  Ok(PluginMcpTarget {
    server_id: server_id.to_string(),
    tool_name: tool_name.to_string(),
  })
}

pub(super) fn mcp_server_for_target(
  command: &HostPluginCommandEntry,
  plugin_root: &Path,
  server_id: &str,
) -> PluginRunnerRunResult<PluginMcpServerManifest> {
  let manifest = read_plugin_manifest(plugin_root)
    .map_err(|failure| PluginRunnerFailure::from_pair(failure).boxed())?;
  let Some(server) = manifest
    .mcp_servers
    .into_iter()
    .find(|server| server.id == server_id)
  else {
    return Err(
      PluginRunnerFailure::empty(
        -32053,
        format!(
          "Plugin command `{}` references MCP server `{}` that is not declared.",
          command.command_id, server_id
        ),
      )
      .boxed(),
    );
  };
  let transport = server.transport.as_deref().unwrap_or("stdio");
  if transport != "stdio" {
    return Err(
      PluginRunnerFailure::empty(
        -32053,
        format!(
          "Plugin command `{}` requires an MCP stdio server.",
          command.command_id
        ),
      )
      .boxed(),
    );
  }

  Ok(server)
}

pub(super) fn mcp_tool_call_payload(
  command: &HostPluginCommandEntry,
  execution: &PluginCommandExecutionEntry,
  target: &PluginMcpTarget,
  thread_id: &str,
  workspace: Option<&WorkspaceSummary>,
  input: Option<&str>,
  connector_refs: &[PluginConnectorExecutionRef],
) -> String {
  let initialize = json!({
    "jsonrpc": "2.0",
    "id": MCP_INITIALIZE_REQUEST_ID,
    "method": "initialize",
    "params": {
      "protocolVersion": "2025-06-18",
      "capabilities": {},
      "clientInfo": {
        "name": "pith",
        "version": "0.1.0"
      }
    }
  });
  let initialized = json!({
    "jsonrpc": "2.0",
    "method": "notifications/initialized",
    "params": {}
  });
  let tool_call = json!({
    "jsonrpc": "2.0",
    "id": MCP_TOOL_CALL_REQUEST_ID,
    "method": "tools/call",
    "params": {
      "name": &target.tool_name,
      "arguments": {
        "envelope": execution.input.envelope,
        "threadId": thread_id,
        "commandId": command.command_id,
        "input": input,
        "workspace": workspace,
        "connectors": connector_refs,
      }
    }
  });

  format!("{initialize}\n{initialized}\n{tool_call}\n")
}

fn read_plugin_manifest(plugin_root: &Path) -> std::result::Result<PluginManifest, (i32, String)> {
  let manifest_path = plugin_root.join("pith-plugin.json");
  let mut file = File::open(&manifest_path).map_err(|error| {
    (
      -32054,
      format!("Plugin manifest could not be read: {error}"),
    )
  })?;
  let mut content = String::new();
  file
    .by_ref()
    .take((PLUGIN_MANIFEST_MAX_BYTES + 1) as u64)
    .read_to_string(&mut content)
    .map_err(|error| {
      (
        -32054,
        format!("Plugin manifest could not be read: {error}"),
      )
    })?;
  if content.len() > PLUGIN_MANIFEST_MAX_BYTES {
    return Err((
      -32054,
      format!(
        "Plugin manifest {} exceeds the {} byte limit.",
        manifest_path.display(),
        PLUGIN_MANIFEST_MAX_BYTES
      ),
    ));
  }
  serde_json::from_str::<PluginManifest>(&content).map_err(|error| {
    (
      -32054,
      format!("Plugin manifest could not be parsed: {error}"),
    )
  })
}
