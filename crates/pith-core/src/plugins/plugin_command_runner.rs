use std::collections::HashMap;
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use pith_model_runtime::GenerationCancellation;
use pith_plugin_host::PluginCommandEntry as HostPluginCommandEntry;
use pith_process::{
  join_bounded_pipe_reader, read_bounded_pipe_in_background, terminate_process_group_or_child,
  wait_for_child, BoundedPipeOutput, ChildExitReason, ChildWaitResult,
};
use pith_protocol::{TimelineItem, WorkspaceSummary};
use serde::Deserialize;
use serde_json::json;

use super::plugin_command_runner_sandbox::PluginRunnerSandbox;

const PLUGIN_RUNNER_TIMEOUT: Duration = Duration::from_secs(60);
const PLUGIN_RUNNER_POLL_INTERVAL: Duration = Duration::from_millis(25);
const PLUGIN_RUNNER_GRACE_PERIOD: Duration = Duration::from_millis(250);
const PLUGIN_RUNNER_OUTPUT_LIMIT: usize = 64 * 1024;
const PLUGIN_RUNNER_LOG_PREVIEW_LIMIT: usize = 2048;

pub(super) struct PluginRunnerResult {
  pub(super) execution_kind: String,
  pub(super) content: String,
  pub(super) items: Vec<TimelineItem>,
  pub(super) attributes: HashMap<String, String>,
}

pub(super) struct PluginRunnerFailure {
  pub(super) code: i32,
  pub(super) message: String,
  pub(super) stdout: String,
  pub(super) stderr: String,
  pub(super) attributes: HashMap<String, String>,
}

struct PluginRunnerProcessOutput {
  stdout: String,
  attributes: HashMap<String, String>,
}

impl PluginRunnerFailure {
  fn empty(code: i32, message: String) -> Self {
    Self::with_output(code, message, String::new(), String::new(), HashMap::new())
  }

  fn new(code: i32, message: String, attributes: HashMap<String, String>) -> Self {
    Self::with_output(code, message, String::new(), String::new(), attributes)
  }

  fn with_output(
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

  fn from_pair((code, message): (i32, String)) -> Self {
    Self::empty(code, message)
  }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PluginRunnerOutputEnvelope {
  content: Option<String>,
  message: Option<String>,
  #[serde(default)]
  items: Vec<PluginRunnerTimelineItemEnvelope>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PluginRunnerTimelineItemEnvelope {
  kind: String,
  title: String,
  content: String,
  #[serde(default)]
  attributes: HashMap<String, String>,
}

pub(crate) fn is_supported_external_plugin_execution(command: &HostPluginCommandEntry) -> bool {
  command.execution.as_ref().is_some_and(|execution| {
    execution.driver == "stdio"
      && execution
        .entrypoint
        .as_deref()
        .is_some_and(|entrypoint| !entrypoint.trim().is_empty())
  })
}

pub(super) fn run_external_plugin_command(
  command: &HostPluginCommandEntry,
  thread_id: &str,
  workspace: Option<&WorkspaceSummary>,
  input: Option<&str>,
  cancellation: &GenerationCancellation,
) -> std::result::Result<PluginRunnerResult, PluginRunnerFailure> {
  if cancellation.is_cancelled() {
    return Err(PluginRunnerFailure::empty(
      -32055,
      format!("Plugin command `{}` was cancelled.", command.command_id),
    ));
  }

  let execution = command.execution.as_ref().ok_or_else(|| {
    PluginRunnerFailure::empty(
      -32053,
      format!(
        "Plugin command `{}` requires an explicit execution contract.",
        command.command_id
      ),
    )
  })?;
  if execution.driver != "stdio" {
    return Err(unsupported_execution_error(command));
  }
  let entrypoint = execution
    .entrypoint
    .as_deref()
    .ok_or_else(|| unsupported_execution_error(command))?;
  let plugin_root = plugin_root_for_command(command).map_err(PluginRunnerFailure::from_pair)?;
  let entrypoint_path =
    safe_entrypoint_path(&plugin_root, entrypoint).map_err(PluginRunnerFailure::from_pair)?;
  let sandbox = PluginRunnerSandbox::prepare(workspace, &command.plugin_id, &plugin_root)
    .map_err(PluginRunnerFailure::from_pair)?;
  let sandbox_attributes = sandbox.attributes();
  let input_payload = json!({
    "envelope": execution.input.envelope,
    "threadId": thread_id,
    "commandId": command.command_id,
    "input": input,
    "workspace": workspace,
  });

  let output = run_stdio_runner(
    command,
    &sandbox,
    &entrypoint_path,
    &input_payload.to_string(),
    cancellation,
    &sandbox_attributes,
  )?;
  Ok(plugin_runner_output(
    command,
    &execution.kind,
    &output.stdout,
    merged_attributes(sandbox_attributes, output.attributes),
  ))
}

fn run_stdio_runner(
  command: &HostPluginCommandEntry,
  sandbox: &PluginRunnerSandbox,
  entrypoint_path: &Path,
  input_payload: &str,
  cancellation: &GenerationCancellation,
  sandbox_attributes: &HashMap<String, String>,
) -> std::result::Result<PluginRunnerProcessOutput, PluginRunnerFailure> {
  let mut process = sandbox.build_command(entrypoint_path);
  process
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .env("PITH_PLUGIN_COMMAND_ID", &command.command_id)
    .env("PITH_PLUGIN_ID", &command.plugin_id)
    .env("PITH_PLUGIN_SANDBOX_DETAIL", sandbox.detail())
    .env("PITH_PLUGIN_SOURCE_PATH", &command.source_path);

  let mut child = process.spawn().map_err(|error| {
    PluginRunnerFailure::new(
      -32054,
      format!(
        "Plugin command `{}` failed to start: {error}",
        command.command_id
      ),
      sandbox_attributes.clone(),
    )
  })?;
  if let Some(mut stdin) = child.stdin.take() {
    if let Err(error) = stdin
      .write_all(input_payload.as_bytes())
      .and_then(|_| stdin.write_all(b"\n"))
    {
      terminate_process_group_or_child(&mut child, PLUGIN_RUNNER_GRACE_PERIOD);
      let _ = child.wait();
      return Err(PluginRunnerFailure::new(
        -32054,
        format!(
          "Plugin command `{}` failed to receive input: {error}",
          command.command_id
        ),
        sandbox_attributes.clone(),
      ));
    }
  }

  let stdout_reader = child
    .stdout
    .take()
    .map(|reader| read_bounded_pipe_in_background(reader, PLUGIN_RUNNER_OUTPUT_LIMIT));
  let stderr_reader = child
    .stderr
    .take()
    .map(|reader| read_bounded_pipe_in_background(reader, PLUGIN_RUNNER_OUTPUT_LIMIT));
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
      sandbox_attributes.clone(),
    )
  })?;
  let stdout_output = join_bounded_pipe_reader(stdout_reader);
  let stderr_output = join_bounded_pipe_reader(stderr_reader);
  let stdout = String::from_utf8_lossy(&stdout_output.bytes)
    .trim()
    .to_string();
  let stderr = String::from_utf8_lossy(&stderr_output.bytes)
    .trim()
    .to_string();
  let mut output_attributes = runner_output_attributes(&wait, &stdout_output, &stderr_output);
  insert_log_preview(
    &mut output_attributes,
    "pluginRunnerStdoutPreview",
    &stdout,
  );
  insert_log_preview(
    &mut output_attributes,
    "pluginRunnerStderrPreview",
    &stderr,
  );
  let failure_attributes =
    merged_attributes(sandbox_attributes.clone(), output_attributes.clone());

  match wait.reason {
    ChildExitReason::Cancelled => {
      return Err(PluginRunnerFailure::with_output(
        -32055,
        format!("Plugin command `{}` was cancelled.", command.command_id),
        stdout,
        stderr,
        failure_attributes,
      ))
    }
    ChildExitReason::TimedOut => {
      return Err(PluginRunnerFailure::with_output(
        -32056,
        format!(
          "Plugin command `{}` timed out after {} seconds.",
          command.command_id,
          PLUGIN_RUNNER_TIMEOUT.as_secs()
        ),
        stdout,
        stderr,
        failure_attributes,
      ))
    }
    ChildExitReason::Completed => {}
  }
  if !wait.status.success() {
    return Err(PluginRunnerFailure::with_output(
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
    ));
  }

  Ok(PluginRunnerProcessOutput {
    stdout,
    attributes: output_attributes,
  })
}

fn plugin_root_for_command(
  command: &HostPluginCommandEntry,
) -> std::result::Result<PathBuf, (i32, String)> {
  Path::new(&command.source_path)
    .parent()
    .and_then(Path::parent)
    .map(Path::to_path_buf)
    .ok_or_else(|| {
      (
        -32054,
        format!(
          "Plugin command `{}` does not have a valid plugin root.",
          command.command_id
        ),
      )
    })
}

fn safe_entrypoint_path(
  plugin_root: &Path,
  entrypoint: &str,
) -> std::result::Result<PathBuf, (i32, String)> {
  let entrypoint_path = Path::new(entrypoint.trim());
  if entrypoint_path.is_absolute()
    || entrypoint_path.components().any(|component| {
      matches!(
        component,
        Component::ParentDir | Component::RootDir | Component::Prefix(_)
      )
    })
  {
    return Err((
      -32054,
      "Plugin runner entrypoint must stay inside the plugin bundle.".to_string(),
    ));
  }

  let root = plugin_root.canonicalize().map_err(|error| {
    (
      -32054,
      format!("Plugin root could not be resolved: {error}"),
    )
  })?;
  let candidate = plugin_root.join(entrypoint_path);
  let resolved = candidate.canonicalize().map_err(|error| {
    (
      -32054,
      format!("Plugin runner entrypoint could not be resolved: {error}"),
    )
  })?;
  if !resolved.starts_with(&root) {
    return Err((
      -32054,
      "Plugin runner entrypoint resolved outside the plugin bundle.".to_string(),
    ));
  }

  Ok(resolved)
}

fn plugin_runner_output(
  command: &HostPluginCommandEntry,
  execution_kind: &str,
  output: &str,
  attributes: HashMap<String, String>,
) -> PluginRunnerResult {
  let Ok(envelope) = serde_json::from_str::<PluginRunnerOutputEnvelope>(output) else {
    return PluginRunnerResult {
      execution_kind: execution_kind.to_string(),
      content: plugin_runner_content(output),
      items: vec![],
      attributes,
    };
  };
  let content = envelope
    .content
    .or(envelope.message)
    .map(|content| content.trim().to_string())
    .filter(|content| !content.is_empty())
    .unwrap_or_else(|| plugin_runner_content(output));
  let items = envelope
    .items
    .into_iter()
    .filter_map(|item| plugin_runner_timeline_item(command, execution_kind, &attributes, item))
    .collect();

  PluginRunnerResult {
    execution_kind: execution_kind.to_string(),
    content,
    items,
    attributes,
  }
}

fn plugin_runner_content(output: &str) -> String {
  if output.trim().is_empty() {
    return "Plugin command completed without output.".to_string();
  }

  output.to_string()
}

fn plugin_runner_timeline_item(
  command: &HostPluginCommandEntry,
  execution_kind: &str,
  base_attributes: &HashMap<String, String>,
  item: PluginRunnerTimelineItemEnvelope,
) -> Option<TimelineItem> {
  let kind = item.kind.trim();
  let title = item.title.trim();
  let content = item.content.trim();
  if kind.is_empty() || title.is_empty() || content.is_empty() {
    return None;
  }

  let mut attributes = base_attributes.clone();
  attributes.extend(item.attributes);
  attributes
    .entry("pluginId".to_string())
    .or_insert_with(|| command.plugin_id.clone());
  attributes
    .entry("commandId".to_string())
    .or_insert_with(|| command.command_id.clone());
  attributes
    .entry("executionKind".to_string())
    .or_insert_with(|| execution_kind.to_string());
  attributes
    .entry("sourcePath".to_string())
    .or_insert_with(|| command.source_path.clone());

  Some(TimelineItem {
    kind: kind.to_string(),
    title: title.to_string(),
    content: content.to_string(),
    attributes: Some(attributes),
  })
}

fn unsupported_execution_error(command: &HostPluginCommandEntry) -> PluginRunnerFailure {
  PluginRunnerFailure::empty(
    -32053,
    format!(
      "Plugin command `{}` requires a supported execution contract.",
      command.command_id
    ),
  )
}

fn runner_output_attributes(
  wait: &ChildWaitResult,
  stdout: &BoundedPipeOutput,
  stderr: &BoundedPipeOutput,
) -> HashMap<String, String> {
  HashMap::from([
    (
      "pluginRunnerExitReason".to_string(),
      child_exit_reason_label(wait.reason).to_string(),
    ),
    ("pluginRunnerExitStatus".to_string(), wait.status.to_string()),
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

fn insert_log_preview(attributes: &mut HashMap<String, String>, key: &str, content: &str) {
  if content.trim().is_empty() {
    return;
  }

  attributes.insert(key.to_string(), bounded_log_preview(content));
}

fn bounded_log_preview(content: &str) -> String {
  let mut preview = content
    .chars()
    .take(PLUGIN_RUNNER_LOG_PREVIEW_LIMIT)
    .collect::<String>();
  if content.chars().count() > PLUGIN_RUNNER_LOG_PREVIEW_LIMIT {
    preview.push_str("\n[truncated]");
  }
  preview
}

fn merged_attributes(
  mut base: HashMap<String, String>,
  attributes: HashMap<String, String>,
) -> HashMap<String, String> {
  base.extend(attributes);
  base
}

fn stderr_suffix(stderr: &str) -> String {
  if stderr.is_empty() {
    String::new()
  } else {
    format!(" Stderr: {stderr}")
  }
}
