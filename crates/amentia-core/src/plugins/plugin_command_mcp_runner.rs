use std::collections::HashMap;

use amentia_model_runtime::GenerationCancellation;
use amentia_plugin_host::{
  PluginCommandEntry as HostPluginCommandEntry, PluginCommandExecutionEntry,
};
use amentia_protocol::WorkspaceSummary;

use super::plugin_command_mcp_output::mcp_runner_output;
use super::plugin_command_mcp_session::{run_mcp_stdio_session, PluginMcpSessionRequest};
use super::plugin_command_mcp_target::{
  insert_mcp_runner_attributes, mcp_server_for_target, mcp_target_for_execution,
  mcp_tool_call_payload,
};
use super::plugin_command_runner::{
  merged_attributes, plugin_runner_setup_failed_attributes, plugin_runner_setup_phase_attributes,
  PluginRunnerFailure, PluginRunnerResult, PluginRunnerRunResult,
};
use super::plugin_command_runner_sandbox::PluginRunnerSandbox;
use super::plugin_command_runner_setup::{
  command_allows_network, insert_connector_runner_attributes, insert_plugin_root_attribute,
  insert_resolved_entrypoint_attribute, insert_runner_input_value_attributes,
  plugin_root_for_command, plugin_runner_setup_attributes, runner_entrypoint_setup_blocker,
  safe_entrypoint_path,
};
use super::plugin_command_types::PluginConnectorExecutionRef;

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
    .map_err(|failure| merge_setup_failure(&setup_attributes, "mcpTargetResolve", *failure))?;
  let plugin_root = plugin_root_for_command(command).map_err(|failure| {
    PluginRunnerFailure::from_pair_with_attributes(
      failure,
      plugin_runner_setup_phase_attributes(&setup_attributes, "pluginRootResolve"),
    )
    .boxed()
  })?;
  insert_plugin_root_attribute(&mut setup_attributes, &plugin_root);
  let server = mcp_server_for_target(command, &plugin_root, &target.server_id)
    .map_err(|failure| merge_setup_failure(&setup_attributes, "mcpServerResolve", *failure))?;
  let server_command = server.command.as_deref().ok_or_else(|| {
    PluginRunnerFailure::with_output(
      -32053,
      format!(
        "Plugin command `{}` requires an MCP server command.",
        command.command_id
      ),
      String::new(),
      String::new(),
      plugin_runner_setup_failed_attributes(plugin_runner_setup_phase_attributes(
        &setup_attributes,
        "mcpServerCommandResolve",
      )),
    )
    .boxed()
  })?;
  setup_attributes.insert("mcpServerCommand".to_string(), server_command.to_string());
  let entrypoint_path = safe_entrypoint_path(&plugin_root, server_command).map_err(|failure| {
    PluginRunnerFailure::from_pair_with_attributes(
      failure,
      plugin_runner_setup_phase_attributes(&setup_attributes, "entrypointResolve"),
    )
    .boxed()
  })?;
  insert_resolved_entrypoint_attribute(&mut setup_attributes, &entrypoint_path);
  let sandbox_setup_attributes =
    plugin_runner_setup_phase_attributes(&setup_attributes, "sandboxPrepare");
  let sandbox = PluginRunnerSandbox::prepare(
    workspace,
    &command.plugin_id,
    &plugin_root,
    command_allows_network(command),
  )
  .map_err(|failure| {
    PluginRunnerFailure::from_pair_with_attributes(failure, sandbox_setup_attributes.clone())
      .boxed()
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

  let output = run_mcp_stdio_session(PluginMcpSessionRequest {
    command,
    sandbox: &sandbox,
    entrypoint_path: &entrypoint_path,
    args: &server.args,
    input_payload: &input_payload,
    connector_refs,
    cancellation,
    sandbox_attributes: &runner_context_attributes,
    target: &target,
  })?;
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
  phase: &str,
  failure: PluginRunnerFailure,
) -> Box<PluginRunnerFailure> {
  PluginRunnerFailure::with_output(
    failure.code,
    failure.message,
    failure.stdout,
    failure.stderr,
    plugin_runner_setup_failed_attributes(merged_attributes(
      plugin_runner_setup_phase_attributes(setup_attributes, phase),
      failure.attributes,
    )),
  )
  .boxed()
}
