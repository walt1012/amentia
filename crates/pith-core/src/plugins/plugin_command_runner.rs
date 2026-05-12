use std::collections::HashMap;
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

use pith_model_runtime::GenerationCancellation;
use pith_plugin_host::PluginCommandEntry as HostPluginCommandEntry;
use pith_process::{
  configure_process_group, join_bounded_pipe_reader, read_bounded_pipe_in_background,
  terminate_process_group_or_child, wait_for_child, ChildExitReason,
};
use pith_protocol::{TimelineItem, WorkspaceSummary};
use serde::Deserialize;
use serde_json::json;

const PLUGIN_RUNNER_TIMEOUT: Duration = Duration::from_secs(60);
const PLUGIN_RUNNER_POLL_INTERVAL: Duration = Duration::from_millis(25);
const PLUGIN_RUNNER_GRACE_PERIOD: Duration = Duration::from_millis(250);
const PLUGIN_RUNNER_OUTPUT_LIMIT: usize = 64 * 1024;

pub(super) struct PluginRunnerResult {
  pub(super) execution_kind: String,
  pub(super) content: String,
  pub(super) items: Vec<TimelineItem>,
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
  command
    .execution
    .as_ref()
    .is_some_and(|execution| {
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
) -> std::result::Result<PluginRunnerResult, (i32, String)> {
  if cancellation.is_cancelled() {
    return Err((
      -32055,
      format!("Plugin command `{}` was cancelled.", command.command_id),
    ));
  }

  let execution = command.execution.as_ref().ok_or_else(|| {
    (
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
  let plugin_root = plugin_root_for_command(command)?;
  let entrypoint_path = safe_entrypoint_path(&plugin_root, entrypoint)?;
  let input_payload = json!({
    "envelope": execution.input.envelope,
    "threadId": thread_id,
    "commandId": command.command_id,
    "input": input,
    "workspace": workspace,
  });

  let output_text = run_stdio_runner(
    command,
    &plugin_root,
    &entrypoint_path,
    &input_payload.to_string(),
    cancellation,
  )?;
  Ok(plugin_runner_output(command, &execution.kind, &output_text))
}

fn run_stdio_runner(
  command: &HostPluginCommandEntry,
  plugin_root: &Path,
  entrypoint_path: &Path,
  input_payload: &str,
  cancellation: &GenerationCancellation,
) -> std::result::Result<String, (i32, String)> {
  let mut process = Command::new(entrypoint_path);
  configure_process_group(&mut process);
  process
    .current_dir(plugin_root)
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .env("PITH_PLUGIN_COMMAND_ID", &command.command_id)
    .env("PITH_PLUGIN_ID", &command.plugin_id)
    .env("PITH_PLUGIN_SOURCE_PATH", &command.source_path);

  let mut child = process.spawn().map_err(|error| {
    (
      -32054,
      format!(
        "Plugin command `{}` failed to start: {error}",
        command.command_id
      ),
    )
  })?;
  if let Some(mut stdin) = child.stdin.take() {
    if let Err(error) = stdin
      .write_all(input_payload.as_bytes())
      .and_then(|_| stdin.write_all(b"\n"))
    {
      terminate_process_group_or_child(&mut child, PLUGIN_RUNNER_GRACE_PERIOD);
      let _ = child.wait();
      return Err((
        -32054,
        format!(
          "Plugin command `{}` failed to receive input: {error}",
          command.command_id
        ),
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
    (
      -32054,
      format!(
        "Plugin command `{}` failed while running: {error}",
        command.command_id
      ),
    )
  })?;
  let stdout = String::from_utf8_lossy(&join_bounded_pipe_reader(stdout_reader).bytes)
    .trim()
    .to_string();
  let stderr = String::from_utf8_lossy(&join_bounded_pipe_reader(stderr_reader).bytes)
    .trim()
    .to_string();

  match wait.reason {
    ChildExitReason::Cancelled => {
      return Err((
        -32055,
        format!("Plugin command `{}` was cancelled.", command.command_id),
      ))
    }
    ChildExitReason::TimedOut => {
      return Err((
        -32056,
        format!(
          "Plugin command `{}` timed out after {} seconds.",
          command.command_id,
          PLUGIN_RUNNER_TIMEOUT.as_secs()
        ),
      ))
    }
    ChildExitReason::Completed => {}
  }
  if !wait.status.success() {
    return Err((
      -32054,
      format!(
        "Plugin command `{}` exited with status {}.{}",
        command.command_id,
        wait.status,
        stderr_suffix(&stderr)
      ),
    ));
  }

  Ok(stdout)
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
) -> PluginRunnerResult {
  let Ok(envelope) = serde_json::from_str::<PluginRunnerOutputEnvelope>(output) else {
    return PluginRunnerResult {
      execution_kind: execution_kind.to_string(),
      content: plugin_runner_content(output),
      items: vec![],
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
    .filter_map(|item| plugin_runner_timeline_item(command, execution_kind, item))
    .collect();

  PluginRunnerResult {
    execution_kind: execution_kind.to_string(),
    content,
    items,
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
  item: PluginRunnerTimelineItemEnvelope,
) -> Option<TimelineItem> {
  let kind = item.kind.trim();
  let title = item.title.trim();
  let content = item.content.trim();
  if kind.is_empty() || title.is_empty() || content.is_empty() {
    return None;
  }

  let mut attributes = item.attributes;
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

fn unsupported_execution_error(command: &HostPluginCommandEntry) -> (i32, String) {
  (
    -32053,
    format!(
      "Plugin command `{}` requires a supported execution contract.",
      command.command_id
    ),
  )
}

fn stderr_suffix(stderr: &str) -> String {
  if stderr.is_empty() {
    String::new()
  } else {
    format!(" Stderr: {stderr}")
  }
}
