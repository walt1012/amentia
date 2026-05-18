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

use super::plugin_command_runner_output::plugin_runner_output;
use super::plugin_command_runner::{
  command_allows_network, insert_connector_runner_attributes, insert_plugin_root_attribute,
  insert_resolved_entrypoint_attribute, insert_runner_input_value_attributes, merged_attributes,
  plugin_root_for_command, plugin_runner_setup_attributes, plugin_runner_setup_failed_attributes,
  run_stdio_runner, runner_entrypoint_setup_blocker, safe_entrypoint_path, PluginRunnerFailure,
  PluginRunnerResult, PluginRunnerRunResult,
};
use super::plugin_command_runner_sandbox::PluginRunnerSandbox;
use super::plugin_command_types::PluginConnectorExecutionRef;

const PLUGIN_MANIFEST_MAX_BYTES: usize = 64 * 1024;
const MCP_INITIALIZE_REQUEST_ID: i64 = 1;
const MCP_TOOL_CALL_REQUEST_ID: i64 = 2;
const MCP_INVALID_JSON_PREVIEW_LIMIT: usize = 240;

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

struct PluginMcpOutputScan {
  tool_response: Option<PluginMcpJsonRpcEnvelope>,
  initialize_response_seen: bool,
  json_response_count: usize,
  invalid_json_line_count: usize,
  output_line_count: usize,
  last_invalid_json_preview: Option<String>,
}

struct PluginMcpContentStats {
  total_count: usize,
  text_count: usize,
  usable_text_count: usize,
  unsupported_count: usize,
  unsupported_types: Vec<String>,
}

pub(super) fn is_supported_mcp_execution(
  command: &HostPluginCommandEntry,
  execution: &PluginCommandExecutionEntry,
) -> bool {
  mcp_target_for_execution(command, execution).is_ok()
}

pub(crate) fn mcp_runner_setup_blocker(command: &HostPluginCommandEntry) -> Option<String> {
  let execution = command.execution.as_ref()?;
  if execution.driver != "mcp" {
    return None;
  }
  let target = match mcp_target_for_execution(command, execution) {
    Ok(target) => target,
    Err(failure) => return Some(failure.message.clone()),
  };
  let plugin_root = match plugin_root_for_command(command) {
    Ok(plugin_root) => plugin_root,
    Err((_, message)) => return Some(message),
  };
  let server = match mcp_server_for_target(command, &plugin_root, &target.server_id) {
    Ok(server) => server,
    Err(failure) => return Some(failure.message.clone()),
  };
  let Some(server_command) = server
    .command
    .as_deref()
    .map(str::trim)
    .filter(|command| !command.is_empty())
  else {
    return Some(format!(
      "Plugin command `{}` requires an MCP server command.",
      command.command_id
    ));
  };
  let entrypoint_path = match safe_entrypoint_path(&plugin_root, server_command) {
    Ok(entrypoint_path) => entrypoint_path,
    Err((_, message)) => return Some(message),
  };

  runner_entrypoint_setup_blocker(command, &entrypoint_path)
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

  let mut setup_attributes = plugin_runner_setup_attributes(command, execution);
  let target = mcp_target_for_execution(command, execution)
    .map_err(|failure| merge_setup_failure(&setup_attributes, *failure))?;
  let plugin_root = plugin_root_for_command(command).map_err(|failure| {
    PluginRunnerFailure::from_pair_with_attributes(failure, setup_attributes.clone()).boxed()
  })?;
  insert_plugin_root_attribute(&mut setup_attributes, &plugin_root);
  let server = mcp_server_for_target(command, &plugin_root, &target.server_id)
    .map_err(|failure| merge_setup_failure(&setup_attributes, *failure))?;
  let server_command = server.command.as_deref().ok_or_else(|| {
    PluginRunnerFailure::with_output(
      -32053,
      format!(
        "Plugin command `{}` requires an MCP server command.",
        command.command_id
      ),
      String::new(),
      String::new(),
      plugin_runner_setup_failed_attributes(setup_attributes.clone()),
    )
    .boxed()
  })?;
  setup_attributes.insert("mcpServerCommand".to_string(), server_command.to_string());
  let entrypoint_path = safe_entrypoint_path(&plugin_root, server_command).map_err(|failure| {
    PluginRunnerFailure::from_pair_with_attributes(failure, setup_attributes.clone()).boxed()
  })?;
  insert_resolved_entrypoint_attribute(&mut setup_attributes, &entrypoint_path);
  let sandbox = PluginRunnerSandbox::prepare(
    workspace,
    &command.plugin_id,
    &plugin_root,
    command_allows_network(command),
  )
  .map_err(|failure| {
    PluginRunnerFailure::from_pair_with_attributes(failure, setup_attributes.clone()).boxed()
  })?;
  let mut runner_context_attributes = setup_attributes;
  runner_context_attributes.extend(sandbox.attributes());
  insert_runner_input_value_attributes(&mut runner_context_attributes, input);
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
    connector_refs,
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

fn merge_setup_failure(
  setup_attributes: &HashMap<String, String>,
  failure: PluginRunnerFailure,
) -> Box<PluginRunnerFailure> {
  PluginRunnerFailure::with_output(
    failure.code,
    failure.message,
    failure.stdout,
    failure.stderr,
    plugin_runner_setup_failed_attributes(merged_attributes(
      setup_attributes.clone(),
      failure.attributes,
    )),
  )
  .boxed()
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
  mut attributes: HashMap<String, String>,
) -> PluginRunnerRunResult<PluginRunnerResult> {
  let mut scan = scan_mcp_output(output);
  scan.insert_attributes(&mut attributes);
  let Some(response) = scan.tool_response.take() else {
    attributes.insert(
      "mcpProtocolStatus".to_string(),
      "missingToolResponse".to_string(),
    );
    return Err(
      PluginRunnerFailure::with_output(
        -32054,
        missing_mcp_tool_response_message(command, target, &scan),
        output.to_string(),
        String::new(),
        attributes,
      )
      .boxed(),
    );
  };
  if let Some(error) = response.error {
    attributes.insert("mcpProtocolStatus".to_string(), "toolError".to_string());
    attributes.insert("mcpErrorCode".to_string(), error.code.to_string());
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
    attributes.insert("mcpProtocolStatus".to_string(), "missingResult".to_string());
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
  if result.is_error {
    let content = mcp_result_content(&result);
    attributes.insert(
      "mcpProtocolStatus".to_string(),
      "toolResultError".to_string(),
    );
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
  let content_stats = mcp_content_stats(&result);
  content_stats.insert_attributes(&mut attributes);
  if content_stats.total_count == 0 && result.structured_content.is_none() {
    attributes.insert("mcpProtocolStatus".to_string(), "emptyResult".to_string());
    return Err(
      PluginRunnerFailure::with_output(
        -32054,
        format!("MCP tool `{}` returned an empty result.", target.tool_name),
        output.to_string(),
        String::new(),
        attributes,
      )
      .boxed(),
    );
  }
  if content_stats.usable_text_count == 0
    && result.structured_content.is_none()
    && content_stats.total_count > 0
  {
    let status = if content_stats.unsupported_count > 0 {
      "unsupportedContent"
    } else {
      "emptyContent"
    };
    attributes.insert("mcpProtocolStatus".to_string(), status.to_string());
    return Err(
      PluginRunnerFailure::with_output(
        -32054,
        unsupported_mcp_content_message(target, &content_stats),
        output.to_string(),
        String::new(),
        attributes,
      )
      .boxed(),
    );
  }
  attributes.insert(
    "mcpProtocolStatus".to_string(),
    mcp_success_protocol_status(&scan).to_string(),
  );
  if let Some(structured_content) = result.structured_content.as_ref() {
    if mcp_structured_content_looks_like_pith_output(structured_content) {
      attributes.insert(
        "mcpStructuredContentStatus".to_string(),
        "pithOutputEnvelope".to_string(),
      );
      attributes.insert(
        "mcpResultSource".to_string(),
        "structuredContent".to_string(),
      );
      let output = structured_content.to_string();
      return plugin_runner_output(command, execution_kind, &output, attributes);
    }

    attributes.insert(
      "mcpStructuredContentStatus".to_string(),
      "generic".to_string(),
    );
  }
  let content = mcp_result_content(&result);
  attributes.insert(
    "mcpResultSource".to_string(),
    mcp_result_content_source(&result, &content_stats).to_string(),
  );
  if mcp_text_content_looks_like_pith_output(&content) {
    attributes.insert(
      "mcpContentStatus".to_string(),
      "pithOutputEnvelope".to_string(),
    );
    return plugin_runner_output(command, execution_kind, &content, attributes);
  }

  Ok(PluginRunnerResult {
    execution_kind: execution_kind.to_string(),
    content,
    items: vec![],
    memory_notes: vec![],
    attributes,
  })
}

fn mcp_structured_content_looks_like_pith_output(value: &Value) -> bool {
  let Some(object) = value.as_object() else {
    return false;
  };

  if object.contains_key("items") || object.contains_key("memoryNotes") {
    return true;
  }

  object.get("content").is_some_and(Value::is_string)
    || object.get("message").is_some_and(Value::is_string)
}

fn mcp_text_content_looks_like_pith_output(text: &str) -> bool {
  let text = text.trim();
  if text.is_empty() {
    return false;
  }
  serde_json::from_str::<Value>(text)
    .ok()
    .is_some_and(|value| mcp_structured_content_looks_like_pith_output(&value))
}

impl PluginMcpContentStats {
  fn insert_attributes(&self, attributes: &mut HashMap<String, String>) {
    attributes.insert("mcpContentCount".to_string(), self.total_count.to_string());
    attributes.insert(
      "mcpTextContentCount".to_string(),
      self.text_count.to_string(),
    );
    attributes.insert(
      "mcpUsableTextContentCount".to_string(),
      self.usable_text_count.to_string(),
    );
    attributes.insert(
      "mcpUnsupportedContentCount".to_string(),
      self.unsupported_count.to_string(),
    );
    if !self.unsupported_types.is_empty() {
      attributes.insert(
        "mcpUnsupportedContentTypes".to_string(),
        self.unsupported_types.join(", "),
      );
    }
  }
}

fn mcp_content_stats(result: &PluginMcpToolResultEnvelope) -> PluginMcpContentStats {
  let mut unsupported_types = vec![];
  let mut text_count = 0;
  let mut usable_text_count = 0;
  let mut unsupported_count = 0;
  for content in &result.content {
    if content.content_type == "text" {
      text_count += 1;
      if content
        .text
        .as_deref()
        .map(str::trim)
        .is_some_and(|text| !text.is_empty())
      {
        usable_text_count += 1;
      }
      continue;
    }
    unsupported_count += 1;
    if !unsupported_types.contains(&content.content_type) {
      unsupported_types.push(content.content_type.clone());
    }
  }

  PluginMcpContentStats {
    total_count: result.content.len(),
    text_count,
    usable_text_count,
    unsupported_count,
    unsupported_types,
  }
}

fn unsupported_mcp_content_message(
  target: &PluginMcpTarget,
  stats: &PluginMcpContentStats,
) -> String {
  if stats.unsupported_count > 0 {
    return format!(
      "MCP tool `{}` returned unsupported non-text content.",
      target.tool_name
    );
  }

  format!(
    "MCP tool `{}` returned empty text content.",
    target.tool_name
  )
}

impl PluginMcpOutputScan {
  fn empty() -> Self {
    Self {
      tool_response: None,
      initialize_response_seen: false,
      json_response_count: 0,
      invalid_json_line_count: 0,
      output_line_count: 0,
      last_invalid_json_preview: None,
    }
  }

  fn insert_attributes(&self, attributes: &mut HashMap<String, String>) {
    attributes.insert(
      "mcpInitializeResponseSeen".to_string(),
      self.initialize_response_seen.to_string(),
    );
    attributes.insert(
      "mcpToolResponseSeen".to_string(),
      self.tool_response.is_some().to_string(),
    );
    attributes.insert(
      "mcpJsonResponseCount".to_string(),
      self.json_response_count.to_string(),
    );
    attributes.insert(
      "mcpInvalidJsonLineCount".to_string(),
      self.invalid_json_line_count.to_string(),
    );
    attributes.insert(
      "mcpOutputLineCount".to_string(),
      self.output_line_count.to_string(),
    );
    if let Some(preview) = self.last_invalid_json_preview.as_ref() {
      attributes.insert("mcpLastInvalidJsonPreview".to_string(), preview.clone());
    }
  }
}

fn scan_mcp_output(output: &str) -> PluginMcpOutputScan {
  let mut scan = PluginMcpOutputScan::empty();
  for line in output
    .lines()
    .map(str::trim)
    .filter(|line| !line.is_empty())
  {
    scan.output_line_count += 1;
    match serde_json::from_str::<PluginMcpJsonRpcEnvelope>(line) {
      Ok(response) => {
        scan.json_response_count += 1;
        let response_id = response.id.as_ref();
        if is_mcp_initialize_response_id(response_id) {
          scan.initialize_response_seen = true;
        }
        if is_mcp_tool_response_id(response_id) {
          scan.tool_response = Some(response);
        }
      }
      Err(_) => {
        scan.invalid_json_line_count += 1;
        scan.last_invalid_json_preview = Some(bounded_mcp_invalid_json_preview(line));
      }
    }
  }
  scan
}

fn missing_mcp_tool_response_message(
  command: &HostPluginCommandEntry,
  target: &PluginMcpTarget,
  scan: &PluginMcpOutputScan,
) -> String {
  if scan.output_line_count == 0 {
    return format!(
      "MCP server `{}` did not write a response for plugin command `{}`.",
      target.server_id, command.command_id
    );
  }
  if scan.json_response_count == 0 {
    return format!(
      "MCP server `{}` wrote {} non-JSON stdout line(s) and no tool response for `{}`.",
      target.server_id, scan.invalid_json_line_count, target.tool_name
    );
  }
  if scan.initialize_response_seen {
    return format!(
      "MCP server `{}` initialized but did not return a tool response for `{}`.",
      target.server_id, target.tool_name
    );
  }
  format!(
    "MCP server `{}` returned {} JSON-RPC response(s), but none matched tool call id {} for `{}`.",
    target.server_id, scan.json_response_count, MCP_TOOL_CALL_REQUEST_ID, target.tool_name
  )
}

fn mcp_success_protocol_status(scan: &PluginMcpOutputScan) -> &'static str {
  if scan.invalid_json_line_count > 0 || !scan.initialize_response_seen {
    "completedWithWarnings"
  } else {
    "completed"
  }
}

fn is_mcp_initialize_response_id(id: Option<&Value>) -> bool {
  is_mcp_response_id(id, MCP_INITIALIZE_REQUEST_ID)
}

fn is_mcp_tool_response_id(id: Option<&Value>) -> bool {
  is_mcp_response_id(id, MCP_TOOL_CALL_REQUEST_ID)
}

fn is_mcp_response_id(id: Option<&Value>, expected_id: i64) -> bool {
  match id {
    Some(Value::Number(number)) => number.as_i64() == Some(expected_id),
    Some(Value::String(value)) => value == &expected_id.to_string(),
    _ => false,
  }
}

fn bounded_mcp_invalid_json_preview(line: &str) -> String {
  let mut preview = line
    .chars()
    .take(MCP_INVALID_JSON_PREVIEW_LIMIT)
    .collect::<String>();
  if line.chars().count() > MCP_INVALID_JSON_PREVIEW_LIMIT {
    preview.push_str("[truncated]");
  }
  preview
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

fn mcp_result_content_source(
  result: &PluginMcpToolResultEnvelope,
  stats: &PluginMcpContentStats,
) -> &'static str {
  if stats.usable_text_count > 0 {
    return "textContent";
  }
  if result.structured_content.is_some() {
    return "structuredContent";
  }
  "emptyResult"
}

#[cfg(test)]
mod tests {
  use serde_json::json;

  use super::{
    mcp_structured_content_looks_like_pith_output, mcp_text_content_looks_like_pith_output,
  };

  #[test]
  fn detects_pith_structured_content_envelopes() {
    assert!(mcp_structured_content_looks_like_pith_output(&json!({
      "content": "Captured context."
    })));
    assert!(mcp_structured_content_looks_like_pith_output(&json!({
      "items": []
    })));
    assert!(mcp_structured_content_looks_like_pith_output(&json!({
      "memoryNotes": []
    })));
  }

  #[test]
  fn leaves_generic_structured_content_as_generic() {
    assert!(!mcp_structured_content_looks_like_pith_output(&json!({
      "content": { "pageId": "abc123" }
    })));
    assert!(!mcp_structured_content_looks_like_pith_output(&json!({
      "databaseId": "db123",
      "properties": { "title": "Task" }
    })));
  }

  #[test]
  fn detects_pith_text_content_envelopes() {
    assert!(mcp_text_content_looks_like_pith_output(
      r#"{"content":"Captured context."}"#
    ));
    assert!(mcp_text_content_looks_like_pith_output(r#"{"items":[]}"#));
    assert!(!mcp_text_content_looks_like_pith_output(
      r#"{"content":{"pageId":"abc123"}}"#
    ));
    assert!(!mcp_text_content_looks_like_pith_output(
      "Captured context."
    ));
  }
}
