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

use super::plugin_command_runner::{merged_attributes, PluginRunnerFailure, PluginRunnerRunResult};
use super::plugin_command_runner_output::bounded_log_preview;
use super::plugin_command_runner_sandbox::PluginRunnerSandbox;
use super::plugin_command_runner_setup::validate_runner_entrypoint;
use super::plugin_command_types::PluginConnectorExecutionRef;

pub(super) const PLUGIN_RUNNER_TIMEOUT: Duration = Duration::from_secs(60);
pub(super) const PLUGIN_RUNNER_POLL_INTERVAL: Duration = Duration::from_millis(25);
pub(super) const PLUGIN_RUNNER_GRACE_PERIOD: Duration = Duration::from_millis(250);
pub(super) const PLUGIN_RUNNER_OUTPUT_LIMIT: usize = 64 * 1024;

pub(super) struct PluginRunnerProcessOutput {
  pub(super) stdout: String,
  pub(super) attributes: HashMap<String, String>,
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

pub(super) fn stderr_suffix(stderr: &str) -> String {
  if stderr.is_empty() {
    String::new()
  } else {
    format!(" Stderr: {stderr}")
  }
}
