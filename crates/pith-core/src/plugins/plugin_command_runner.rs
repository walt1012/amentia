use std::collections::HashMap;
use std::path::Path;
use std::process::Stdio;
use std::time::Duration;

use pith_model_runtime::GenerationCancellation;
use pith_plugin_host::PluginCommandEntry as HostPluginCommandEntry;
use pith_process::{
  join_bounded_pipe_reader, join_pipe_writer, read_bounded_pipe_in_background, wait_for_child,
  write_pipe_in_background, BoundedPipeOutput, ChildExitReason, ChildWaitResult, PipeWriteOutput,
};
use pith_protocol::{TimelineItem, WorkspaceSummary};
use serde_json::json;

use super::plugin_command_mcp_runner::{is_supported_mcp_execution, run_mcp_plugin_command};
use super::plugin_command_runner_output::{bounded_log_preview, plugin_runner_output};
use super::plugin_command_runner_sandbox::PluginRunnerSandbox;
use super::plugin_command_runner_setup::{
  command_allows_network, insert_connector_runner_attributes, insert_plugin_root_attribute,
  insert_resolved_entrypoint_attribute, insert_runner_input_value_attributes,
  plugin_root_for_command, plugin_runner_setup_attributes, safe_entrypoint_path,
  unsupported_execution_error, validate_runner_entrypoint,
};
use super::plugin_command_types::{PluginConnectorExecutionRef, PluginRunnerMemoryNoteDraft};

pub(super) const PLUGIN_RUNNER_TIMEOUT: Duration = Duration::from_secs(60);
pub(super) const PLUGIN_RUNNER_POLL_INTERVAL: Duration = Duration::from_millis(25);
pub(super) const PLUGIN_RUNNER_GRACE_PERIOD: Duration = Duration::from_millis(250);
pub(super) const PLUGIN_RUNNER_OUTPUT_LIMIT: usize = 64 * 1024;
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

pub(super) struct PluginRunnerProcessOutput {
  pub(super) stdout: String,
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

pub(super) fn run_stdio_runner(
  command: &HostPluginCommandEntry,
  sandbox: &PluginRunnerSandbox,
  entrypoint_path: &Path,
  args: &[String],
  input_payload: &str,
  connector_refs: &[PluginConnectorExecutionRef],
  cancellation: &GenerationCancellation,
  sandbox_attributes: &HashMap<String, String>,
) -> PluginRunnerRunResult<PluginRunnerProcessOutput> {
  let mut runner_attributes = sandbox_attributes.clone();
  validate_runner_entrypoint(command, entrypoint_path, &mut runner_attributes)?;

  let mut process = sandbox.build_command(entrypoint_path);
  process.args(args);
  process
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .env("PITH_PLUGIN_COMMAND_ID", &command.command_id)
    .env("PITH_PLUGIN_ID", &command.plugin_id)
    .env("PITH_PLUGIN_SANDBOX_DETAIL", sandbox.detail())
    .env("PITH_PLUGIN_SOURCE_PATH", &command.source_path);
  for connector in connector_refs {
    let Some(secret) = connector.credential_secret.as_deref() else {
      continue;
    };
    let Some(env_key) = connector.credential_provider.env_key.as_deref() else {
      continue;
    };
    process.env(env_key, secret);
  }

  let mut child = process.spawn().map_err(|error| {
    PluginRunnerFailure::new(
      -32054,
      format!(
        "Plugin command `{}` failed to start: {error}",
        command.command_id
      ),
      runner_attributes.clone(),
    )
    .boxed()
  })?;

  let stdout_reader = child
    .stdout
    .take()
    .map(|reader| read_bounded_pipe_in_background(reader, PLUGIN_RUNNER_OUTPUT_LIMIT));
  let stderr_reader = child
    .stderr
    .take()
    .map(|reader| read_bounded_pipe_in_background(reader, PLUGIN_RUNNER_OUTPUT_LIMIT));
  let stdin_writer = child
    .stdin
    .take()
    .map(|writer| write_pipe_in_background(writer, runner_input_bytes(input_payload)));
  let wait = wait_for_child(
    &mut child,
    PLUGIN_RUNNER_TIMEOUT,
    PLUGIN_RUNNER_POLL_INTERVAL,
    PLUGIN_RUNNER_GRACE_PERIOD,
    || cancellation.is_cancelled(),
  )
  .map_err(|error| {
    PluginRunnerFailure::new(
      -32054,
      format!(
        "Plugin command `{}` failed while running: {error}",
        command.command_id
      ),
      runner_attributes.clone(),
    )
    .boxed()
  })?;
  let stdin_output = join_pipe_writer(stdin_writer);
  let stdout_output = join_bounded_pipe_reader(stdout_reader);
  let stderr_output = join_bounded_pipe_reader(stderr_reader);
  let stdout = String::from_utf8_lossy(&stdout_output.bytes)
    .trim()
    .to_string();
  let stderr = String::from_utf8_lossy(&stderr_output.bytes)
    .trim()
    .to_string();
  let mut output_attributes = runner_output_attributes(&wait, &stdout_output, &stderr_output);
  insert_stdin_writer_attributes(&mut output_attributes, &stdin_output);
  insert_log_preview(&mut output_attributes, "pluginRunnerStdoutPreview", &stdout);
  insert_log_preview(&mut output_attributes, "pluginRunnerStderrPreview", &stderr);
  let failure_attributes = merged_attributes(runner_attributes.clone(), output_attributes.clone());

  match wait.reason {
    ChildExitReason::Cancelled => {
      return Err(
        PluginRunnerFailure::with_output(
          -32055,
          format!("Plugin command `{}` was cancelled.", command.command_id),
          stdout,
          stderr,
          failure_attributes,
        )
        .boxed(),
      )
    }
    ChildExitReason::TimedOut => {
      return Err(
        PluginRunnerFailure::with_output(
          -32056,
          format!(
            "Plugin command `{}` timed out after {} seconds.",
            command.command_id,
            PLUGIN_RUNNER_TIMEOUT.as_secs()
          ),
          stdout,
          stderr,
          failure_attributes,
        )
        .boxed(),
      )
    }
    ChildExitReason::Completed => {}
  }
  if let Some(error) = stdin_output.error_message {
    return Err(
      PluginRunnerFailure::with_output(
        -32054,
        format!(
          "Plugin command `{}` failed to receive input: {error}",
          command.command_id
        ),
        stdout,
        stderr,
        failure_attributes,
      )
      .boxed(),
    );
  }
  if !wait.status.success() {
    return Err(
      PluginRunnerFailure::with_output(
        -32054,
        format!(
          "Plugin command `{}` exited with status {}.{}",
          command.command_id,
          wait.status,
          stderr_suffix(&stderr)
        ),
        stdout,
        stderr,
        failure_attributes,
      )
      .boxed(),
    );
  }

  Ok(PluginRunnerProcessOutput {
    stdout,
    attributes: merged_attributes(runner_attributes, output_attributes),
  })
}

fn runner_input_bytes(input_payload: &str) -> Vec<u8> {
  let mut input = Vec::with_capacity(input_payload.len() + 1);
  input.extend_from_slice(input_payload.as_bytes());
  input.push(b'\n');
  input
}

pub(super) fn insert_stdin_writer_attributes(
  attributes: &mut HashMap<String, String>,
  output: &PipeWriteOutput,
) {
  attributes.insert(
    "pluginRunnerStdinBytesWritten".to_string(),
    output.bytes_written.to_string(),
  );
  attributes.insert(
    "pluginRunnerStdinWriteStatus".to_string(),
    if output.error_message.is_some() {
      "failed".to_string()
    } else {
      "completed".to_string()
    },
  );
  if let Some(error) = output.error_message.as_ref() {
    attributes.insert("pluginRunnerStdinWriteError".to_string(), error.clone());
  }
}

pub(super) fn plugin_runner_input_bytes(input_payload: &str) -> Vec<u8> {
  runner_input_bytes(input_payload)
}

pub(super) fn runner_output_attributes(
  wait: &ChildWaitResult,
  stdout: &BoundedPipeOutput,
  stderr: &BoundedPipeOutput,
) -> HashMap<String, String> {
  HashMap::from([
    (
      "pluginRunnerExitReason".to_string(),
      child_exit_reason_label(wait.reason).to_string(),
    ),
    (
      "pluginRunnerExitStatus".to_string(),
      wait.status.to_string(),
    ),
    (
      "pluginRunnerExitCode".to_string(),
      wait
        .status
        .code()
        .map(|code| code.to_string())
        .unwrap_or_else(|| "unknown".to_string()),
    ),
    (
      "pluginRunnerStdoutRetainedBytes".to_string(),
      stdout.bytes.len().to_string(),
    ),
    (
      "pluginRunnerStdoutSourceBytes".to_string(),
      stdout.source_byte_count.to_string(),
    ),
    (
      "pluginRunnerStdoutTruncated".to_string(),
      (stdout.bytes.len() < stdout.source_byte_count).to_string(),
    ),
    (
      "pluginRunnerStderrRetainedBytes".to_string(),
      stderr.bytes.len().to_string(),
    ),
    (
      "pluginRunnerStderrSourceBytes".to_string(),
      stderr.source_byte_count.to_string(),
    ),
    (
      "pluginRunnerStderrTruncated".to_string(),
      (stderr.bytes.len() < stderr.source_byte_count).to_string(),
    ),
  ])
}

fn child_exit_reason_label(reason: ChildExitReason) -> &'static str {
  match reason {
    ChildExitReason::Completed => "completed",
    ChildExitReason::TimedOut => "timedOut",
    ChildExitReason::Cancelled => "cancelled",
  }
}

pub(super) fn insert_log_preview(
  attributes: &mut HashMap<String, String>,
  key: &str,
  content: &str,
) {
  if content.trim().is_empty() {
    return;
  }

  attributes.insert(key.to_string(), bounded_log_preview(content));
}

pub(super) fn merged_attributes(
  mut base: HashMap<String, String>,
  attributes: HashMap<String, String>,
) -> HashMap<String, String> {
  base.extend(attributes);
  base
}

pub(super) fn stderr_suffix(stderr: &str) -> String {
  if stderr.is_empty() {
    String::new()
  } else {
    format!(" Stderr: {stderr}")
  }
}
