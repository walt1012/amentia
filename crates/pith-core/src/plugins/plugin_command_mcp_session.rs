use std::collections::HashMap;
use std::io::Read;
use std::path::Path;
use std::process::{ExitStatus, Stdio};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};
use std::time::Instant;

use pith_model_runtime::GenerationCancellation;
use pith_plugin_host::PluginCommandEntry as HostPluginCommandEntry;
use pith_process::{
  join_bounded_pipe_reader, join_pipe_writer, read_bounded_pipe_in_background,
  terminate_process_group_or_child, write_pipe_in_background, BoundedPipeOutput,
};

use super::plugin_command_mcp_output::PluginMcpOutputScan;
use super::plugin_command_mcp_target::PluginMcpTarget;
use super::plugin_command_runner::{
  insert_log_preview, insert_stdin_writer_attributes, merged_attributes, plugin_runner_input_bytes,
  stderr_suffix, PluginRunnerFailure, PluginRunnerProcessOutput, PluginRunnerRunResult,
  PLUGIN_RUNNER_GRACE_PERIOD, PLUGIN_RUNNER_OUTPUT_LIMIT, PLUGIN_RUNNER_POLL_INTERVAL,
  PLUGIN_RUNNER_TIMEOUT,
};
use super::plugin_command_runner_sandbox::PluginRunnerSandbox;
use super::plugin_command_runner_setup::validate_runner_entrypoint;
use super::plugin_command_types::PluginConnectorExecutionRef;

pub(super) struct PluginMcpSessionRequest<'a> {
  pub(super) command: &'a HostPluginCommandEntry,
  pub(super) sandbox: &'a PluginRunnerSandbox,
  pub(super) entrypoint_path: &'a Path,
  pub(super) args: &'a [String],
  pub(super) input_payload: &'a str,
  pub(super) connector_refs: &'a [PluginConnectorExecutionRef],
  pub(super) cancellation: &'a GenerationCancellation,
  pub(super) sandbox_attributes: &'a HashMap<String, String>,
  pub(super) target: &'a PluginMcpTarget,
}

pub(super) fn run_mcp_stdio_session(
  request: PluginMcpSessionRequest<'_>,
) -> PluginRunnerRunResult<PluginRunnerProcessOutput> {
  let command = request.command;
  let target = request.target;
  let mut runner_attributes = request.sandbox_attributes.clone();
  validate_runner_entrypoint(command, request.entrypoint_path, &mut runner_attributes)?;

  let mut process = request.sandbox.build_command(request.entrypoint_path);
  process.args(request.args);
  process
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .env("PITH_PLUGIN_COMMAND_ID", &command.command_id)
    .env("PITH_PLUGIN_ID", &command.plugin_id)
    .env("PITH_PLUGIN_SANDBOX_DETAIL", request.sandbox.detail())
    .env("PITH_PLUGIN_SOURCE_PATH", &command.source_path);
  for connector in request.connector_refs {
    let Some(secret) = connector.credential_secret.as_deref() else {
      continue;
    };
    let Some(env_key) = connector.credential_provider.env_key.as_deref() else {
      continue;
    };
    process.env(env_key, secret);
  }

  let mut child = process.spawn().map_err(|error| {
    PluginRunnerFailure::with_output(
      -32054,
      format!(
        "Plugin command `{}` failed to start MCP server `{}`: {error}",
        command.command_id, target.server_id
      ),
      String::new(),
      String::new(),
      runner_attributes.clone(),
    )
    .boxed()
  })?;

  let (stdout_line_sender, stdout_lines) = mpsc::channel();
  let stdout_reader = child
    .stdout
    .take()
    .map(|reader| read_mcp_stdout_lines_in_background(reader, stdout_line_sender));
  let stderr_reader = child
    .stderr
    .take()
    .map(|reader| read_bounded_pipe_in_background(reader, PLUGIN_RUNNER_OUTPUT_LIMIT));
  let stdin_writer = child.stdin.take().map(|writer| {
    write_pipe_in_background(writer, plugin_runner_input_bytes(request.input_payload))
  });

  let wait = wait_for_mcp_tool_response(&mut child, &stdout_lines, request.cancellation).map_err(
    |error| {
      PluginRunnerFailure::with_output(
        -32054,
        format!(
          "Plugin command `{}` failed while running MCP server `{}`: {error}",
          command.command_id, target.server_id
        ),
        String::new(),
        String::new(),
        runner_attributes.clone(),
      )
      .boxed()
    },
  )?;
  let stdin_output = join_pipe_writer(stdin_writer);
  let stdout_output = join_bounded_pipe_reader(stdout_reader);
  let stderr_output = join_bounded_pipe_reader(stderr_reader);
  let stdout = String::from_utf8_lossy(&stdout_output.bytes)
    .trim()
    .to_string();
  let stderr = String::from_utf8_lossy(&stderr_output.bytes)
    .trim()
    .to_string();
  let mut output_attributes = mcp_session_output_attributes(&wait, &stdout_output, &stderr_output);
  insert_stdin_writer_attributes(&mut output_attributes, &stdin_output);
  insert_log_preview(&mut output_attributes, "pluginRunnerStdoutPreview", &stdout);
  insert_log_preview(&mut output_attributes, "pluginRunnerStderrPreview", &stderr);
  let failure_attributes = merged_attributes(runner_attributes.clone(), output_attributes.clone());

  match wait.reason {
    PluginMcpSessionStopReason::Cancelled => {
      return Err(
        PluginRunnerFailure::with_output(
          -32055,
          format!(
            "Plugin command `{}` MCP server `{}` was cancelled.",
            command.command_id, target.server_id
          ),
          stdout,
          stderr,
          failure_attributes,
        )
        .boxed(),
      )
    }
    PluginMcpSessionStopReason::TimedOut => {
      return Err(
        PluginRunnerFailure::with_output(
          -32056,
          format!(
            "Plugin command `{}` MCP server `{}` timed out after {} seconds.",
            command.command_id,
            target.server_id,
            PLUGIN_RUNNER_TIMEOUT.as_secs()
          ),
          stdout,
          stderr,
          failure_attributes,
        )
        .boxed(),
      )
    }
    PluginMcpSessionStopReason::ToolResponse => {}
    PluginMcpSessionStopReason::ChildExit => {
      if let Some(error) = stdin_output.error_message {
        return Err(
          PluginRunnerFailure::with_output(
            -32054,
            format!(
              "Plugin command `{}` MCP server `{}` failed to receive input: {error}",
              command.command_id, target.server_id
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
              "Plugin command `{}` MCP server `{}` exited with status {}.{}",
              command.command_id,
              target.server_id,
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
    }
  }

  Ok(PluginRunnerProcessOutput {
    stdout,
    attributes: merged_attributes(runner_attributes, output_attributes),
  })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PluginMcpSessionStopReason {
  ToolResponse,
  ChildExit,
  TimedOut,
  Cancelled,
}

struct PluginMcpSessionWait {
  status: ExitStatus,
  reason: PluginMcpSessionStopReason,
}

fn wait_for_mcp_tool_response(
  child: &mut std::process::Child,
  stdout_lines: &Receiver<String>,
  cancellation: &GenerationCancellation,
) -> std::io::Result<PluginMcpSessionWait> {
  let started_at = Instant::now();
  let mut scan = PluginMcpOutputScan::empty();

  loop {
    drain_mcp_stdout_lines(stdout_lines, &mut scan);
    if scan.has_tool_response() {
      terminate_process_group_or_child(child, PLUGIN_RUNNER_GRACE_PERIOD);
      return Ok(PluginMcpSessionWait {
        status: child.wait()?,
        reason: PluginMcpSessionStopReason::ToolResponse,
      });
    }

    if let Some(status) = child.try_wait()? {
      return Ok(PluginMcpSessionWait {
        status,
        reason: PluginMcpSessionStopReason::ChildExit,
      });
    }

    if cancellation.is_cancelled() {
      terminate_process_group_or_child(child, PLUGIN_RUNNER_GRACE_PERIOD);
      return Ok(PluginMcpSessionWait {
        status: child.wait()?,
        reason: PluginMcpSessionStopReason::Cancelled,
      });
    }

    if started_at.elapsed() >= PLUGIN_RUNNER_TIMEOUT {
      terminate_process_group_or_child(child, PLUGIN_RUNNER_GRACE_PERIOD);
      return Ok(PluginMcpSessionWait {
        status: child.wait()?,
        reason: PluginMcpSessionStopReason::TimedOut,
      });
    }

    thread::sleep(PLUGIN_RUNNER_POLL_INTERVAL);
  }
}

fn drain_mcp_stdout_lines(receiver: &Receiver<String>, scan: &mut PluginMcpOutputScan) {
  while let Ok(line) = receiver.try_recv() {
    scan.observe_line(&line);
  }
}

fn read_mcp_stdout_lines_in_background<R>(
  mut reader: R,
  sender: Sender<String>,
) -> JoinHandle<BoundedPipeOutput>
where
  R: Read + Send + 'static,
{
  thread::spawn(move || {
    let mut bytes = Vec::with_capacity(PLUGIN_RUNNER_OUTPUT_LIMIT.min(64 * 1024));
    let mut source_byte_count = 0;
    let mut buffer = [0_u8; 8192];
    let mut line = Vec::new();
    let mut line_truncated = false;

    while let Ok(bytes_read) = reader.read(&mut buffer) {
      if bytes_read == 0 {
        break;
      }

      source_byte_count += bytes_read;
      let remaining_output = PLUGIN_RUNNER_OUTPUT_LIMIT.saturating_sub(bytes.len());
      if remaining_output > 0 {
        bytes.extend_from_slice(&buffer[..bytes_read.min(remaining_output)]);
      }

      for byte in &buffer[..bytes_read] {
        if line.len() < PLUGIN_RUNNER_OUTPUT_LIMIT {
          line.push(*byte);
        } else {
          line_truncated = true;
        }
        if *byte == b'\n' {
          send_mcp_stdout_line(&sender, &line, line_truncated);
          line.clear();
          line_truncated = false;
        }
      }
    }

    if !line.is_empty() {
      send_mcp_stdout_line(&sender, &line, line_truncated);
    }

    BoundedPipeOutput {
      bytes,
      source_byte_count,
    }
  })
}

fn send_mcp_stdout_line(sender: &Sender<String>, line: &[u8], truncated: bool) {
  let mut text = String::from_utf8_lossy(line).to_string();
  if truncated {
    text.push_str("[truncated]");
  }
  let _ = sender.send(text);
}

fn mcp_session_output_attributes(
  wait: &PluginMcpSessionWait,
  stdout: &BoundedPipeOutput,
  stderr: &BoundedPipeOutput,
) -> HashMap<String, String> {
  HashMap::from([
    (
      "pluginRunnerExitReason".to_string(),
      mcp_session_stop_reason_label(wait.reason).to_string(),
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

fn mcp_session_stop_reason_label(reason: PluginMcpSessionStopReason) -> &'static str {
  match reason {
    PluginMcpSessionStopReason::ToolResponse => "toolResponse",
    PluginMcpSessionStopReason::ChildExit => "completed",
    PluginMcpSessionStopReason::TimedOut => "timedOut",
    PluginMcpSessionStopReason::Cancelled => "cancelled",
  }
}
