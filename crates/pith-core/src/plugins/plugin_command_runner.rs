use std::collections::HashMap;

use pith_model_runtime::GenerationCancellation;
use pith_plugin_host::PluginCommandEntry as HostPluginCommandEntry;
use pith_protocol::{TimelineItem, WorkspaceSummary};
use serde_json::json;

use super::plugin_command_mcp_runner::{is_supported_mcp_execution, run_mcp_plugin_command};
use super::plugin_command_runner_output::plugin_runner_output;
use super::plugin_command_runner_process::run_stdio_runner;
use super::plugin_command_runner_sandbox::PluginRunnerSandbox;
use super::plugin_command_runner_setup::{
  command_allows_network, insert_connector_runner_attributes, insert_plugin_root_attribute,
  insert_resolved_entrypoint_attribute, insert_runner_input_value_attributes,
  plugin_root_for_command, plugin_runner_setup_attributes, safe_entrypoint_path,
  unsupported_execution_error,
};
use super::plugin_command_types::{PluginConnectorExecutionRef, PluginRunnerMemoryNoteDraft};

const PLUGIN_RUNNER_SETUP_STATUS_KEY: &str = "pluginRunnerSetupStatus";

pub(super) type PluginRunnerRunResult<T> = std::result::Result<T, Box<PluginRunnerFailure>>;

pub(super) struct PluginRunnerResult {
  pub(super) execution_kind: String,
  pub(super) content: String,
  pub(super) items: Vec<TimelineItem>,
  pub(super) memory_notes: Vec<PluginRunnerMemoryNoteDraft>,
  pub(super) attributes: HashMap<String, String>,
}

pub(super) struct PluginRunnerFailure {
  pub(super) code: i32,
  pub(super) message: String,
  pub(super) stdout: String,
  pub(super) stderr: String,
  pub(super) attributes: HashMap<String, String>,
}

impl PluginRunnerFailure {
  pub(super) fn empty(code: i32, message: String) -> Self {
    Self::with_output(code, message, String::new(), String::new(), HashMap::new())
  }

  pub(super) fn new(code: i32, message: String, attributes: HashMap<String, String>) -> Self {
    Self::with_output(code, message, String::new(), String::new(), attributes)
  }

  pub(super) fn with_output(
    code: i32,
    message: String,
    stdout: String,
    stderr: String,
    attributes: HashMap<String, String>,
  ) -> Self {
    Self {
      code,
      message,
      stdout,
      stderr,
      attributes,
    }
  }

  pub(super) fn from_pair((code, message): (i32, String)) -> Self {
    Self::empty(code, message)
  }

  pub(super) fn from_pair_with_attributes(
    (code, message): (i32, String),
    attributes: HashMap<String, String>,
  ) -> Self {
    let mut attributes = plugin_runner_setup_failed_attributes(attributes);
    attributes.insert("pluginRunnerSetupDetail".to_string(), message.clone());
    Self::new(code, message, attributes)
  }

  pub(super) fn boxed(self) -> Box<Self> {
    Box::new(self)
  }
}

pub(super) fn plugin_runner_setup_failed_attributes(
  mut attributes: HashMap<String, String>,
) -> HashMap<String, String> {
  attributes.insert(
    PLUGIN_RUNNER_SETUP_STATUS_KEY.to_string(),
    "failed".to_string(),
  );
  attributes
}

pub(super) fn plugin_runner_setup_phase_attributes(
  attributes: &HashMap<String, String>,
  phase: &str,
) -> HashMap<String, String> {
  let mut attributes = attributes.clone();
  attributes.insert("pluginRunnerSetupPhase".to_string(), phase.to_string());
  attributes
}

pub(crate) fn is_supported_external_plugin_execution(command: &HostPluginCommandEntry) -> bool {
  command.execution.as_ref().is_some_and(|execution| {
    if execution.driver == "stdio" {
      return execution
        .entrypoint
        .as_deref()
        .is_some_and(|entrypoint| !entrypoint.trim().is_empty());
    }
    execution.driver == "mcp" && is_supported_mcp_execution(command, execution)
  })
}

pub(super) fn run_external_plugin_command(
  command: &HostPluginCommandEntry,
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

  let execution = command.execution.as_ref().ok_or_else(|| {
    PluginRunnerFailure::empty(
      -32053,
      format!(
        "Plugin command `{}` requires an explicit execution contract.",
        command.command_id
      ),
    )
    .boxed()
  })?;
  let mut setup_attributes = plugin_runner_setup_attributes(command, execution);
  if execution.driver != "stdio" {
    if execution.driver == "mcp" {
      return run_mcp_plugin_command(
        command,
        execution,
        thread_id,
        workspace,
        input,
        connector_refs,
        cancellation,
      );
    }
    return Err(unsupported_execution_error(command));
  }
  let entrypoint = execution
    .entrypoint
    .as_deref()
    .ok_or_else(|| unsupported_execution_error(command))?;
  let plugin_root = plugin_root_for_command(command).map_err(|failure| {
    PluginRunnerFailure::from_pair_with_attributes(
      failure,
      plugin_runner_setup_phase_attributes(&setup_attributes, "pluginRootResolve"),
    )
    .boxed()
  })?;
  insert_plugin_root_attribute(&mut setup_attributes, &plugin_root);
  let entrypoint_path = safe_entrypoint_path(&plugin_root, entrypoint).map_err(|failure| {
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
  let input_payload = json!({
    "envelope": execution.input.envelope,
    "threadId": thread_id,
    "commandId": command.command_id,
    "input": input,
    "workspace": workspace,
    "connectors": connector_refs,
  });

  let output = run_stdio_runner(
    command,
    &sandbox,
    &entrypoint_path,
    &[],
    &input_payload.to_string(),
    connector_refs,
    cancellation,
    &runner_context_attributes,
  )?;
  plugin_runner_output(
    command,
    &execution.kind,
    &output.stdout,
    merged_attributes(runner_context_attributes, output.attributes),
  )
}

pub(super) fn merged_attributes(
  mut base: HashMap<String, String>,
  attributes: HashMap<String, String>,
) -> HashMap<String, String> {
  base.extend(attributes);
  base
}
