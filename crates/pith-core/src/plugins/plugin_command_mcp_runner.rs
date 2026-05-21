use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::process::{ExitStatus, Stdio};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};
use std::time::Instant;

use pith_model_runtime::GenerationCancellation;
use pith_plugin_host::{
  PluginCommandEntry as HostPluginCommandEntry, PluginCommandExecutionEntry, PluginManifest,
  PluginMcpServerManifest,
};
use pith_process::{
  join_bounded_pipe_reader, join_pipe_writer, read_bounded_pipe_in_background,
  terminate_process_group_or_child, write_pipe_in_background, BoundedPipeOutput,
};
use pith_protocol::WorkspaceSummary;
use serde::Deserialize;
use serde_json::{json, Value};

use super::plugin_command_runner::{
  command_allows_network, insert_connector_runner_attributes, insert_plugin_root_attribute,
  insert_log_preview, insert_resolved_entrypoint_attribute, insert_runner_input_value_attributes,
  insert_stdin_writer_attributes, merged_attributes, plugin_root_for_command,
  plugin_runner_input_bytes, plugin_runner_setup_attributes, plugin_runner_setup_failed_attributes,
  plugin_runner_setup_phase_attributes, runner_entrypoint_setup_blocker, safe_entrypoint_path,
  stderr_suffix, validate_runner_entrypoint, PluginRunnerFailure, PluginRunnerProcessOutput,
  PluginRunnerResult, PluginRunnerRunResult, PLUGIN_RUNNER_GRACE_PERIOD,
  PLUGIN_RUNNER_OUTPUT_LIMIT, PLUGIN_RUNNER_POLL_INTERVAL, PLUGIN_RUNNER_TIMEOUT,
};
use super::plugin_command_runner_output::plugin_runner_output;
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

  let output = run_mcp_stdio_session(
    command,
    &sandbox,
    &entrypoint_path,
    &server.args,
    &input_payload,
    connector_refs,
    cancellation,
    &runner_context_attributes,
    &target,
  )?;
  mcp_runner_output(
    command,
    &execution.kind,
    &target,
    &output.stdout,
    merged_attributes(runner_context_attributes, output.attributes),
  )
}

fn run_mcp_stdio_session(
  command: &HostPluginCommandEntry,
  sandbox: &PluginRunnerSandbox,
  entrypoint_path: &Path,
  args: &[String],
  input_payload: &str,
  connector_refs: &[PluginConnectorExecutionRef],
  cancellation: &GenerationCancellation,
  sandbox_attributes: &HashMap<String, String>,
  target: &PluginMcpTarget,
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
        "Plugin command `{}` failed to start MCP server `{}`: {error}",
        command.command_id, target.server_id
      ),
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
  let stdin_writer = child
    .stdin
    .take()
    .map(|writer| write_pipe_in_background(writer, plugin_runner_input_bytes(input_payload)));

  let wait = wait_for_mcp_tool_response(&mut child, &stdout_lines, cancellation).map_err(|error| {
    PluginRunnerFailure::new(
      -32054,
      format!(
        "Plugin command `{}` failed while running MCP server `{}`: {error}",
        command.command_id, target.server_id
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
    if scan.tool_response.is_some() {
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

  fn observe_line(&mut self, line: &str) {
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
    scan.observe_line(line);
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
