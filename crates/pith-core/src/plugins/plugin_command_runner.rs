use std::collections::HashMap;
use std::fs::Metadata;
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use pith_model_runtime::GenerationCancellation;
use pith_plugin_host::{
  PluginCommandEntry as HostPluginCommandEntry, PluginCommandEnvelopeEntry,
  PluginCommandExecutionEntry,
};
use pith_process::{
  join_bounded_pipe_reader, read_bounded_pipe_in_background, terminate_process_group_or_child,
  wait_for_child, BoundedPipeOutput, ChildExitReason, ChildWaitResult,
};
use pith_protocol::{TimelineItem, WorkspaceSummary};
use serde::Deserialize;
use serde_json::json;

use super::plugin_command_mcp_runner::{is_supported_mcp_execution, run_mcp_plugin_command};
use super::plugin_command_runner_sandbox::PluginRunnerSandbox;
use super::plugin_command_types::{PluginConnectorExecutionRef, PluginRunnerMemoryNoteDraft};

const PLUGIN_RUNNER_TIMEOUT: Duration = Duration::from_secs(60);
const PLUGIN_RUNNER_POLL_INTERVAL: Duration = Duration::from_millis(25);
const PLUGIN_RUNNER_GRACE_PERIOD: Duration = Duration::from_millis(250);
const PLUGIN_RUNNER_OUTPUT_LIMIT: usize = 64 * 1024;
const PLUGIN_RUNNER_LOG_PREVIEW_LIMIT: usize = 2048;
const PLUGIN_RUNNER_MEMORY_NOTE_LIMIT: usize = 4;
const PLUGIN_RUNNER_MEMORY_NOTE_TITLE_LIMIT: usize = 120;
const PLUGIN_RUNNER_MEMORY_NOTE_BODY_LIMIT: usize = 4096;
const PLUGIN_RUNNER_MEMORY_NOTE_TAG_LIMIT: usize = 8;
const PLUGIN_RUNNER_MEMORY_NOTE_TAG_LENGTH_LIMIT: usize = 40;

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

  fn new(code: i32, message: String, attributes: HashMap<String, String>) -> Self {
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
    Self::new(code, message, attributes)
  }

  pub(super) fn boxed(self) -> Box<Self> {
    Box::new(self)
  }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PluginRunnerOutputEnvelope {
  content: Option<String>,
  message: Option<String>,
  #[serde(default)]
  items: Vec<PluginRunnerTimelineItemEnvelope>,
  #[serde(default)]
  memory_notes: Vec<PluginRunnerMemoryNoteEnvelope>,
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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PluginRunnerMemoryNoteEnvelope {
  title: Option<String>,
  body: Option<String>,
  source: Option<String>,
  #[serde(default)]
  tags: Vec<String>,
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

pub(crate) fn stdio_runner_setup_blocker(command: &HostPluginCommandEntry) -> Option<String> {
  let execution = command.execution.as_ref()?;
  if execution.driver != "stdio" {
    return None;
  }
  let entrypoint = execution
    .entrypoint
    .as_deref()
    .map(str::trim)
    .filter(|entrypoint| !entrypoint.is_empty())?;
  let plugin_root = match plugin_root_for_command(command) {
    Ok(plugin_root) => plugin_root,
    Err((_, message)) => return Some(message),
  };
  let entrypoint_path = match safe_entrypoint_path(&plugin_root, entrypoint) {
    Ok(entrypoint_path) => entrypoint_path,
    Err((_, message)) => return Some(message),
  };
  runner_entrypoint_setup_blocker(command, &entrypoint_path)
}

pub(crate) fn runner_entrypoint_setup_blocker(
  command: &HostPluginCommandEntry,
  entrypoint_path: &Path,
) -> Option<String> {
  let metadata = match entrypoint_path.metadata() {
    Ok(metadata) => metadata,
    Err(error) => {
      return Some(format!(
        "Plugin command `{}` runner entrypoint metadata could not be read: {error}",
        command.command_id
      ));
    }
  };
  if !metadata.is_file() {
    return Some(format!(
      "Plugin command `{}` runner entrypoint is not a file: {}",
      command.command_id,
      entrypoint_path.display()
    ));
  }
  if !runner_entrypoint_is_executable(&metadata) {
    return Some(format!(
      "Plugin command `{}` runner entrypoint is not executable: {}",
      command.command_id,
      entrypoint_path.display()
    ));
  }

  None
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
    PluginRunnerFailure::from_pair_with_attributes(failure, setup_attributes.clone()).boxed()
  })?;
  insert_plugin_root_attribute(&mut setup_attributes, &plugin_root);
  let entrypoint_path = safe_entrypoint_path(&plugin_root, entrypoint).map_err(|failure| {
    PluginRunnerFailure::from_pair_with_attributes(failure, setup_attributes.clone()).boxed()
  })?;
  insert_resolved_entrypoint_attribute(&mut setup_attributes, &entrypoint_path);
  let sandbox = PluginRunnerSandbox::prepare(
    workspace,
    &command.plugin_id,
    &plugin_root,
    command_allows_network(command),
  )
  .map_err(|failure| {
    PluginRunnerFailure::from_pair_with_attributes(failure, setup_attributes.clone()).boxed()
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

pub(super) fn plugin_runner_setup_attributes(
  command: &HostPluginCommandEntry,
  execution: &PluginCommandExecutionEntry,
) -> HashMap<String, String> {
  let mut attributes = HashMap::from([
    (
      "pluginRunnerExecutionDriver".to_string(),
      execution.driver.clone(),
    ),
    (
      "pluginRunnerExecutionKind".to_string(),
      execution.kind.clone(),
    ),
    (
      "pluginRunnerSourcePath".to_string(),
      command.source_path.clone(),
    ),
  ]);
  if let Some(entrypoint) = execution
    .entrypoint
    .as_deref()
    .map(str::trim)
    .filter(|entrypoint| !entrypoint.is_empty())
  {
    attributes.insert("pluginRunnerEntrypoint".to_string(), entrypoint.to_string());
  }
  insert_envelope_attributes(&mut attributes, "pluginRunnerInput", &execution.input);
  insert_envelope_attributes(&mut attributes, "pluginRunnerOutput", &execution.output);
  attributes
}

fn insert_envelope_attributes(
  attributes: &mut HashMap<String, String>,
  prefix: &str,
  envelope: &PluginCommandEnvelopeEntry,
) {
  attributes.insert(format!("{prefix}Envelope"), envelope.envelope.clone());
  attributes.insert(
    format!("{prefix}FieldCount"),
    envelope.fields.len().to_string(),
  );
  let field_names = envelope
    .fields
    .iter()
    .map(|field| field.name.as_str())
    .collect::<Vec<_>>()
    .join(", ");
  if !field_names.is_empty() {
    attributes.insert(format!("{prefix}FieldNames"), field_names);
  }
  let required_fields = envelope
    .fields
    .iter()
    .filter(|field| field.required)
    .map(|field| field.name.as_str())
    .collect::<Vec<_>>()
    .join(", ");
  if !required_fields.is_empty() {
    attributes.insert(format!("{prefix}RequiredFields"), required_fields);
  }
}

pub(super) fn insert_runner_input_value_attributes(
  attributes: &mut HashMap<String, String>,
  input: Option<&str>,
) {
  attributes.insert(
    "pluginRunnerInputProvided".to_string(),
    input.is_some().to_string(),
  );
  attributes.insert(
    "pluginRunnerInputBytes".to_string(),
    input.map(str::len).unwrap_or(0).to_string(),
  );
}

pub(super) fn insert_plugin_root_attribute(
  attributes: &mut HashMap<String, String>,
  plugin_root: &Path,
) {
  attributes.insert(
    "pluginRunnerPluginRoot".to_string(),
    plugin_root.display().to_string(),
  );
}

pub(super) fn insert_resolved_entrypoint_attribute(
  attributes: &mut HashMap<String, String>,
  entrypoint_path: &Path,
) {
  attributes.insert(
    "pluginRunnerResolvedEntrypoint".to_string(),
    entrypoint_path.display().to_string(),
  );
}

pub(super) fn insert_connector_runner_attributes(
  attributes: &mut HashMap<String, String>,
  connector_refs: &[PluginConnectorExecutionRef],
) {
  if connector_refs.is_empty() {
    return;
  }

  attributes.insert(
    "pluginRunnerConnectorCount".to_string(),
    connector_refs.len().to_string(),
  );
  attributes.insert(
    "pluginRunnerConnectorIds".to_string(),
    connector_refs
      .iter()
      .map(|connector| connector.connector_id.as_str())
      .collect::<Vec<_>>()
      .join(", "),
  );
  attributes.insert(
    "pluginRunnerConnectorStores".to_string(),
    connector_refs
      .iter()
      .map(|connector| connector.credential_provider.store.as_str())
      .collect::<Vec<_>>()
      .join(", "),
  );
  attributes.insert(
    "pluginRunnerConnectorServices".to_string(),
    connector_refs
      .iter()
      .map(|connector| connector.service.as_str())
      .collect::<Vec<_>>()
      .join(", "),
  );
  attributes.insert(
    "pluginRunnerCredentialProviders".to_string(),
    connector_refs
      .iter()
      .map(|connector| connector.credential_provider.provider.as_str())
      .collect::<Vec<_>>()
      .join(", "),
  );
  attributes.insert(
    "pluginRunnerCredentialHandles".to_string(),
    connector_refs
      .iter()
      .map(|connector| connector.credential_provider.handle.as_str())
      .collect::<Vec<_>>()
      .join(", "),
  );
  attributes.insert(
    "pluginRunnerCredentialLabels".to_string(),
    connector_refs
      .iter()
      .map(|connector| connector.credential_provider.label.as_str())
      .collect::<Vec<_>>()
      .join(", "),
  );
  attributes.insert(
    "pluginRunnerCredentialAuthorizedAt".to_string(),
    connector_refs
      .iter()
      .map(|connector| connector.credential_provider.authorized_at.to_string())
      .collect::<Vec<_>>()
      .join(", "),
  );
  attributes.insert(
    "pluginRunnerSecretBindings".to_string(),
    connector_refs
      .iter()
      .map(PluginConnectorExecutionRef::credential_binding)
      .collect::<Vec<_>>()
      .join(", "),
  );
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
  if let Some(mut stdin) = child.stdin.take() {
    if let Err(error) = stdin
      .write_all(input_payload.as_bytes())
      .and_then(|_| stdin.write_all(b"\n"))
    {
      terminate_process_group_or_child(&mut child, PLUGIN_RUNNER_GRACE_PERIOD);
      let _ = child.wait();
      return Err(
        PluginRunnerFailure::new(
          -32054,
          format!(
            "Plugin command `{}` failed to receive input: {error}",
            command.command_id
          ),
          runner_attributes.clone(),
        )
        .boxed(),
      );
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
      runner_attributes.clone(),
    )
    .boxed()
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

fn validate_runner_entrypoint(
  command: &HostPluginCommandEntry,
  entrypoint_path: &Path,
  attributes: &mut HashMap<String, String>,
) -> PluginRunnerRunResult<()> {
  let metadata = entrypoint_path.metadata().map_err(|error| {
    let mut failure_attributes = attributes.clone();
    failure_attributes.insert(
      "pluginRunnerEntrypointCheck".to_string(),
      "metadataError".to_string(),
    );
    PluginRunnerFailure::new(
      -32054,
      format!(
        "Plugin command `{}` runner entrypoint metadata could not be read: {error}",
        command.command_id
      ),
      failure_attributes,
    )
    .boxed()
  })?;
  let file_kind = runner_entrypoint_file_kind(&metadata);
  attributes.insert(
    "pluginRunnerEntrypointFileKind".to_string(),
    file_kind.to_string(),
  );
  if !metadata.is_file() {
    attributes.insert(
      "pluginRunnerEntrypointExecutable".to_string(),
      "false".to_string(),
    );
    attributes.insert(
      "pluginRunnerEntrypointCheck".to_string(),
      "notFile".to_string(),
    );
    return Err(
      PluginRunnerFailure::new(
        -32054,
        format!(
          "Plugin command `{}` runner entrypoint is not a file: {}",
          command.command_id,
          entrypoint_path.display()
        ),
        attributes.clone(),
      )
      .boxed(),
    );
  }

  let executable = runner_entrypoint_is_executable(&metadata);
  attributes.insert(
    "pluginRunnerEntrypointExecutable".to_string(),
    executable.to_string(),
  );
  if !executable {
    attributes.insert(
      "pluginRunnerEntrypointCheck".to_string(),
      "notExecutable".to_string(),
    );
    return Err(
      PluginRunnerFailure::new(
        -32054,
        format!(
          "Plugin command `{}` runner entrypoint is not executable: {}",
          command.command_id,
          entrypoint_path.display()
        ),
        attributes.clone(),
      )
      .boxed(),
    );
  }

  attributes.insert(
    "pluginRunnerEntrypointCheck".to_string(),
    "ready".to_string(),
  );
  Ok(())
}

fn runner_entrypoint_file_kind(metadata: &Metadata) -> &'static str {
  if metadata.is_file() {
    "file"
  } else if metadata.is_dir() {
    "directory"
  } else {
    "other"
  }
}

#[cfg(unix)]
fn runner_entrypoint_is_executable(metadata: &Metadata) -> bool {
  metadata.permissions().mode() & 0o111 != 0
}

#[cfg(not(unix))]
fn runner_entrypoint_is_executable(_metadata: &Metadata) -> bool {
  true
}

pub(super) fn plugin_root_for_command(
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

pub(super) fn safe_entrypoint_path(
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
  mut attributes: HashMap<String, String>,
) -> PluginRunnerRunResult<PluginRunnerResult> {
  let Ok(envelope) = serde_json::from_str::<PluginRunnerOutputEnvelope>(output) else {
    attributes.insert(
      "pluginRunnerOutputStatus".to_string(),
      "plainText".to_string(),
    );
    return Ok(PluginRunnerResult {
      execution_kind: execution_kind.to_string(),
      content: plugin_runner_content(output),
      items: vec![],
      memory_notes: vec![],
      attributes,
    });
  };
  let content = envelope
    .content
    .or(envelope.message)
    .map(|content| content.trim().to_string())
    .filter(|content| !content.is_empty())
    .unwrap_or_default();
  let (items, invalid_item_count) =
    plugin_runner_timeline_items(command, execution_kind, &attributes, envelope.items);
  let (memory_notes, invalid_memory_note_count) = plugin_runner_memory_notes(envelope.memory_notes);
  insert_plugin_runner_output_attributes(
    &mut attributes,
    &content,
    items.len(),
    invalid_item_count,
    memory_notes.len(),
    invalid_memory_note_count,
  );
  if content.is_empty() && items.is_empty() && memory_notes.is_empty() {
    attributes.insert(
      "pluginRunnerOutputStatus".to_string(),
      "emptyEnvelope".to_string(),
    );
    return Err(
      PluginRunnerFailure::with_output(
        -32054,
        format!(
          "Plugin command `{}` returned an output envelope without content or valid timeline items.",
          command.command_id
        ),
        output.to_string(),
        String::new(),
        attributes,
      )
      .boxed(),
    );
  }
  attributes.insert(
    "pluginRunnerOutputStatus".to_string(),
    "envelope".to_string(),
  );
  let items = plugin_runner_timeline_items_with_attributes(items, &attributes);

  Ok(PluginRunnerResult {
    execution_kind: execution_kind.to_string(),
    content: if content.is_empty() {
      if memory_notes.is_empty() {
        "Plugin command completed with timeline items.".to_string()
      } else {
        "Plugin command completed with memory notes.".to_string()
      }
    } else {
      content
    },
    items,
    memory_notes,
    attributes,
  })
}

fn plugin_runner_timeline_items(
  command: &HostPluginCommandEntry,
  execution_kind: &str,
  base_attributes: &HashMap<String, String>,
  items: Vec<PluginRunnerTimelineItemEnvelope>,
) -> (Vec<TimelineItem>, usize) {
  let total_item_count = items.len();
  let valid_items = items
    .into_iter()
    .filter_map(|item| plugin_runner_timeline_item(command, execution_kind, base_attributes, item))
    .collect::<Vec<_>>();
  let invalid_item_count = total_item_count.saturating_sub(valid_items.len());

  (valid_items, invalid_item_count)
}

fn plugin_runner_timeline_items_with_attributes(
  items: Vec<TimelineItem>,
  attributes: &HashMap<String, String>,
) -> Vec<TimelineItem> {
  items
    .into_iter()
    .map(|mut item| {
      let item_attributes = item.attributes.get_or_insert_with(HashMap::new);
      for (key, value) in attributes {
        item_attributes
          .entry(key.clone())
          .or_insert_with(|| value.clone());
      }
      item
    })
    .collect()
}

fn insert_plugin_runner_output_attributes(
  attributes: &mut HashMap<String, String>,
  content: &str,
  valid_item_count: usize,
  invalid_item_count: usize,
  memory_note_count: usize,
  invalid_memory_note_count: usize,
) {
  attributes.insert("pluginRunnerOutputParsed".to_string(), "true".to_string());
  attributes.insert(
    "pluginRunnerOutputContentBytes".to_string(),
    content.len().to_string(),
  );
  attributes.insert(
    "pluginRunnerOutputValidTimelineItemCount".to_string(),
    valid_item_count.to_string(),
  );
  attributes.insert(
    "pluginRunnerOutputInvalidTimelineItemCount".to_string(),
    invalid_item_count.to_string(),
  );
  attributes.insert(
    "pluginRunnerOutputMemoryNoteCount".to_string(),
    memory_note_count.to_string(),
  );
  attributes.insert(
    "pluginRunnerOutputInvalidMemoryNoteCount".to_string(),
    invalid_memory_note_count.to_string(),
  );
}

fn plugin_runner_memory_notes(
  notes: Vec<PluginRunnerMemoryNoteEnvelope>,
) -> (Vec<PluginRunnerMemoryNoteDraft>, usize) {
  let total_note_count = notes.len();
  let valid_notes = notes
    .into_iter()
    .take(PLUGIN_RUNNER_MEMORY_NOTE_LIMIT)
    .filter_map(plugin_runner_memory_note)
    .collect::<Vec<_>>();
  let invalid_note_count = total_note_count.saturating_sub(valid_notes.len());

  (valid_notes, invalid_note_count)
}

fn plugin_runner_memory_note(
  note: PluginRunnerMemoryNoteEnvelope,
) -> Option<PluginRunnerMemoryNoteDraft> {
  let title = note.title.as_deref().map(str::trim).unwrap_or_default();
  let body = note.body.as_deref().map(str::trim).unwrap_or_default();
  if title.is_empty() || body.is_empty() {
    return None;
  }

  Some(PluginRunnerMemoryNoteDraft {
    title: bounded_runner_memory_text(title, PLUGIN_RUNNER_MEMORY_NOTE_TITLE_LIMIT),
    body: bounded_runner_memory_text(body, PLUGIN_RUNNER_MEMORY_NOTE_BODY_LIMIT),
    source: note
      .source
      .as_deref()
      .map(str::trim)
      .filter(|source| !source.is_empty())
      .map(str::to_string),
    tags: note
      .tags
      .into_iter()
      .take(PLUGIN_RUNNER_MEMORY_NOTE_TAG_LIMIT)
      .map(|tag| tag.trim().to_string())
      .filter(|tag| !tag.is_empty())
      .map(|tag| bounded_runner_memory_text(&tag, PLUGIN_RUNNER_MEMORY_NOTE_TAG_LENGTH_LIMIT))
      .collect(),
  })
}

fn bounded_runner_memory_text(value: &str, limit: usize) -> String {
  value.chars().take(limit).collect()
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

pub(super) fn unsupported_execution_error(
  command: &HostPluginCommandEntry,
) -> Box<PluginRunnerFailure> {
  let attributes = command
    .execution
    .as_ref()
    .map(|execution| plugin_runner_setup_attributes(command, execution))
    .unwrap_or_default();
  PluginRunnerFailure::new(
    -32053,
    format!(
      "Plugin command `{}` requires a supported execution contract.",
      command.command_id
    ),
    attributes,
  )
  .boxed()
}

pub(super) fn command_allows_network(command: &HostPluginCommandEntry) -> bool {
  let declares_network = command
    .permissions
    .iter()
    .any(|permission| permission == "network.outbound");
  if !declares_network {
    return false;
  }

  command
    .execution
    .as_ref()
    .and_then(|execution| execution.connector_ids.as_ref())
    .map(|connector_ids| !connector_ids.is_empty())
    .unwrap_or(true)
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

pub(super) fn merged_attributes(
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
