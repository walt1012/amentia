use std::collections::HashMap;

use pith_plugin_host::PluginCommandEntry as HostPluginCommandEntry;

use super::plugin_command_mcp_output_protocol::{
  mcp_content_stats, mcp_result_content, mcp_result_content_source,
  mcp_structured_content_looks_like_pith_output, mcp_success_protocol_status,
  mcp_text_content_looks_like_pith_output, missing_mcp_tool_response_message, scan_mcp_output,
  unsupported_mcp_content_message,
};
use super::plugin_command_mcp_target::PluginMcpTarget;
use super::plugin_command_runner::{
  PluginRunnerFailure, PluginRunnerResult, PluginRunnerRunResult,
};
use super::plugin_command_runner_output::plugin_runner_output;

pub(super) fn mcp_runner_output(
  command: &HostPluginCommandEntry,
  execution_kind: &str,
  target: &PluginMcpTarget,
  output: &str,
  mut attributes: HashMap<String, String>,
) -> PluginRunnerRunResult<PluginRunnerResult> {
  let mut scan = scan_mcp_output(output);
  scan.insert_attributes(&mut attributes);
  let Some(response) = scan.take_tool_response() else {
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
  if mcp_command_requires_connector_workflow(command) {
    attributes.insert(
      "mcpProtocolStatus".to_string(),
      "missingConnectorWorkflowOutput".to_string(),
    );
    return Err(
      PluginRunnerFailure::with_output(
        -32054,
        format!(
          "MCP command `{}` is bound to a connector workflow and must return a Pith output envelope.",
          command.command_id
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
    memory_notes: vec![],
    attributes,
  })
}

fn mcp_command_requires_connector_workflow(command: &HostPluginCommandEntry) -> bool {
  command
    .execution
    .as_ref()
    .and_then(|execution| execution.workflow_id.as_deref())
    .map(str::trim)
    .is_some_and(|workflow_id| !workflow_id.is_empty())
}
