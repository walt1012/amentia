use std::collections::HashMap;

use pith_plugin_host::PluginCommandEntry as HostPluginCommandEntry;
use serde::Deserialize;
use serde_json::Value;

use super::plugin_command_mcp_target::{
  PluginMcpTarget, MCP_INITIALIZE_REQUEST_ID, MCP_TOOL_CALL_REQUEST_ID,
};
use super::plugin_command_runner::{
  PluginRunnerFailure, PluginRunnerResult, PluginRunnerRunResult,
};
use super::plugin_command_runner_output::plugin_runner_output;

const MCP_INVALID_JSON_PREVIEW_LIMIT: usize = 240;

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

pub(super) struct PluginMcpOutputScan {
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

pub(super) fn mcp_runner_output(
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

impl PluginMcpOutputScan {
  pub(super) fn empty() -> Self {
    Self {
      tool_response: None,
      initialize_response_seen: false,
      json_response_count: 0,
      invalid_json_line_count: 0,
      output_line_count: 0,
      last_invalid_json_preview: None,
    }
  }

  pub(super) fn observe_line(&mut self, line: &str) {
    let line = line.trim();
    if line.is_empty() {
      return;
    }

    self.output_line_count += 1;
    match serde_json::from_str::<PluginMcpJsonRpcEnvelope>(line) {
      Ok(response) => {
        self.json_response_count += 1;
        let response_id = response.id.as_ref();
        if is_mcp_initialize_response_id(response_id) {
          self.initialize_response_seen = true;
        }
        if is_mcp_tool_response_id(response_id) {
          self.tool_response = Some(response);
        }
      }
      Err(_) => {
        self.invalid_json_line_count += 1;
        self.last_invalid_json_preview = Some(bounded_mcp_invalid_json_preview(line));
      }
    }
  }

  pub(super) fn has_tool_response(&self) -> bool {
    self.tool_response.is_some()
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

fn scan_mcp_output(output: &str) -> PluginMcpOutputScan {
  let mut scan = PluginMcpOutputScan::empty();
  for line in output
    .lines()
    .map(str::trim)
    .filter(|line| !line.is_empty())
  {
    scan.observe_line(line);
  }
  scan
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
