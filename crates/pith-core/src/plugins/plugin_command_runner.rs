use std::collections::HashMap;
use std::fs::Metadata;
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
  join_bounded_pipe_reader, join_pipe_writer, read_bounded_pipe_in_background,
  terminate_process_group_or_child, wait_for_child, write_pipe_in_background, BoundedPipeOutput,
  ChildExitReason, ChildWaitResult, PipeWriteOutput,
};
use pith_protocol::{TimelineItem, WorkspaceSummary};
use serde_json::json;

use super::plugin_command_mcp_runner::{is_supported_mcp_execution, run_mcp_plugin_command};
use super::plugin_command_runner_output::{bounded_log_preview, plugin_runner_output};
use super::plugin_command_runner_sandbox::PluginRunnerSandbox;
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
  if connector_refs.len() == 1 {
    attributes.insert(
      "pluginRunnerConnectorId".to_string(),
      connector_refs[0].connector_id.clone(),
    );
  }
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

pub(super) fn validate_runner_entrypoint(
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
