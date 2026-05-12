use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::Path;

use pith_model_runtime::GenerationCancellation;
use pith_plugin_host::{
  PluginCommandEntry as HostPluginCommandEntry, PluginCommandExecutionEntry, PluginManifest,
  PluginMcpServerManifest,
};
use pith_protocol::WorkspaceSummary;
use serde::Deserialize;
use serde_json::{json, Value};

use super::plugin_command_runner::{
  insert_connector_runner_attributes, merged_attributes, plugin_root_for_command, run_stdio_runner,
  safe_entrypoint_path, PluginRunnerFailure, PluginRunnerResult, PluginRunnerRunResult,
};
use super::plugin_command_runner_sandbox::PluginRunnerSandbox;
use super::plugin_command_types::PluginConnectorExecutionRef;

const PLUGIN_MANIFEST_MAX_BYTES: usize = 64 * 1024;
const MCP_TOOL_CALL_REQUEST_ID: i64 = 2;

struct PluginMcpTarget {
  server_id: String,
  tool_name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PluginMcpJsonRpcEnvelope {
  id: Option<Value>,
  result: Option<PluginMcpToolResultEnvelope>,
  error: Option<PluginMcpErrorEnvelope>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PluginMcpToolResultEnvelope {
  #[serde(default)]
  content: Vec<PluginMcpContentEnvelope>,
  #[serde(default)]
  structured_content: Option<Value>,
  #[serde(default)]
  is_error: bool,
}

#[derive(Debug, Deserialize)]
struct PluginMcpContentEnvelope {
  #[serde(rename = "type")]
  content_type: String,
  text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PluginMcpErrorEnvelope {
  code: i32,
  message: String,
}

pub(super) fn is_supported_mcp_execution(
  command: &HostPluginCommandEntry,
  execution: &PluginCommandExecutionEntry,
) -> bool {
  let Ok(target) = mcp_target_for_execution(command, execution) else {
    return false;
  };
  let Ok(plugin_root) = plugin_root_for_command(command) else {
    return false;
  };
  mcp_server_for_target(command, &plugin_root, &target.server_id)
    .ok()
    .and_then(|server| server.command)
    .is_some_and(|command| !command.trim().is_empty())
}

pub(super) fn run_mcp_plugin_command(
  command: &HostPluginCommandEntry,
  execution: &PluginCommandExecutionEntry,
  thread_id: &str,
  workspace: Option<&WorkspaceSummary>,
  input: Option<&str>,
  connector_refs: &[PluginConnectorExecutionRef],
  cancellation: &GenerationCancellation,
) -> PluginRunnerRunResult<PluginRunnerResult> {
  if cancellation.is_cancelled() {
    return Err(
      PluginRunnerFailure::empty(
        -32055,
        format!("Plugin command `{}` was cancelled.", command.command_id),
      )
      .boxed(),
    );
  }

  let target = mcp_target_for_execution(command, execution)?;
  let plugin_root = plugin_root_for_command(command)
    .map_err(|failure| PluginRunnerFailure::from_pair(failure).boxed())?;
  let server = mcp_server_for_target(command, &plugin_root, &target.server_id)?;
  let server_command = server.command.as_deref().ok_or_else(|| {
    PluginRunnerFailure::empty(
      -32053,
      format!(
        "Plugin command `{}` requires an MCP server command.",
        command.command_id
      ),
    )
    .boxed()
  })?;
  let entrypoint_path = safe_entrypoint_path(&plugin_root, server_command)
    .map_err(|failure| PluginRunnerFailure::from_pair(failure).boxed())?;
  let sandbox = PluginRunnerSandbox::prepare(workspace, &command.plugin_id, &plugin_root)
    .map_err(|failure| PluginRunnerFailure::from_pair(failure).boxed())?;
  let mut runner_context_attributes = sandbox.attributes();
  insert_connector_runner_attributes(&mut runner_context_attributes, connector_refs);
  insert_mcp_runner_attributes(&mut runner_context_attributes, &target, &server);
  let input_payload = mcp_tool_call_payload(
    command,
    execution,
    &target,
    thread_id,
    workspace,
    input,
    connector_refs,
  );

  let output = run_stdio_runner(
    command,
    &sandbox,
    &entrypoint_path,
    &server.args,
    &input_payload,
    cancellation,
    &runner_context_attributes,
  )?;
  mcp_runner_output(
    command,
    &execution.kind,
    &target,
    &output.stdout,
    merged_attributes(runner_context_attributes, output.attributes),
  )
}

fn insert_mcp_runner_attributes(
  attributes: &mut HashMap<String, String>,
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

fn mcp_target_for_execution(
  command: &HostPluginCommandEntry,
  execution: &PluginCommandExecutionEntry,
) -> PluginRunnerRunResult<PluginMcpTarget> {
  let entrypoint = execution
    .entrypoint
    .as_deref()
    .map(str::trim)
    .filter(|entrypoint| !entrypoint.is_empty())
    .ok_or_else(|| super::plugin_command_runner::unsupported_execution_error(command))?;
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

fn mcp_server_for_target(
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

fn mcp_tool_call_payload(
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
    "id": 1,
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

fn mcp_runner_output(
  command: &HostPluginCommandEntry,
  execution_kind: &str,
  target: &PluginMcpTarget,
  output: &str,
  attributes: HashMap<String, String>,
) -> PluginRunnerRunResult<PluginRunnerResult> {
  let Some(response) = mcp_tool_response(output) else {
    return Err(
      PluginRunnerFailure::with_output(
        -32054,
        format!(
          "Plugin command `{}` did not return an MCP tool response.",
          command.command_id
        ),
        output.to_string(),
        String::new(),
        attributes,
      )
      .boxed(),
    );
  };
  if let Some(error) = response.error {
    return Err(
      PluginRunnerFailure::with_output(
        -32054,
        format!(
          "MCP tool `{}` failed with code {}: {}",
          target.tool_name, error.code, error.message
        ),
        output.to_string(),
        String::new(),
        attributes,
      )
      .boxed(),
    );
  }
  let Some(result) = response.result else {
    return Err(
      PluginRunnerFailure::with_output(
        -32054,
        format!(
          "Plugin command `{}` returned an MCP response without a result.",
          command.command_id
        ),
        output.to_string(),
        String::new(),
        attributes,
      )
      .boxed(),
    );
  };
  let content = mcp_result_content(&result);
  if result.is_error {
    return Err(
      PluginRunnerFailure::with_output(
        -32054,
        format!(
          "MCP tool `{}` returned an error: {}",
          target.tool_name, content
        ),
        output.to_string(),
        String::new(),
        attributes,
      )
      .boxed(),
    );
  }

  Ok(PluginRunnerResult {
    execution_kind: execution_kind.to_string(),
    content,
    items: vec![],
    attributes,
  })
}

fn mcp_tool_response(output: &str) -> Option<PluginMcpJsonRpcEnvelope> {
  output
    .lines()
    .filter_map(|line| {
      let response = serde_json::from_str::<PluginMcpJsonRpcEnvelope>(line.trim()).ok()?;
      if is_mcp_tool_response_id(response.id.as_ref()) {
        Some(response)
      } else {
        None
      }
    })
    .next_back()
}

fn is_mcp_tool_response_id(id: Option<&Value>) -> bool {
  match id {
    Some(Value::Number(number)) => number.as_i64() == Some(MCP_TOOL_CALL_REQUEST_ID),
    Some(Value::String(value)) => value == &MCP_TOOL_CALL_REQUEST_ID.to_string(),
    _ => false,
  }
}

fn mcp_result_content(result: &PluginMcpToolResultEnvelope) -> String {
  let text = result
    .content
    .iter()
    .filter(|content| content.content_type == "text")
    .filter_map(|content| content.text.as_deref())
    .map(str::trim)
    .filter(|text| !text.is_empty())
    .collect::<Vec<_>>()
    .join("\n");
  if !text.is_empty() {
    return text;
  }
  if let Some(structured_content) = result.structured_content.as_ref() {
    return serde_json::to_string_pretty(structured_content)
      .unwrap_or_else(|_| structured_content.to_string());
  }
  "MCP tool call completed.".to_string()
}
