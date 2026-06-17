use std::collections::HashMap;
use std::fs::Metadata;
use std::path::{Component, Path, PathBuf};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use amentia_plugin_host::{
  PluginCommandEntry as HostPluginCommandEntry, PluginCommandEnvelopeEntry,
  PluginCommandExecutionEntry,
};

use super::plugin_command_runner::{PluginRunnerFailure, PluginRunnerRunResult};
use super::plugin_command_types::PluginConnectorExecutionRef;

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

pub(super) fn runner_entrypoint_setup_blocker(
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
  if let Some(workflow_id) = execution
    .workflow_id
    .as_deref()
    .map(str::trim)
    .filter(|workflow_id| !workflow_id.is_empty())
  {
    attributes.insert(
      "pluginRunnerConnectorWorkflowId".to_string(),
      workflow_id.to_string(),
    );
  }
  if let Some(workflow) = execution.workflow.as_ref() {
    attributes.insert(
      "pluginRunnerConnectorWorkflowName".to_string(),
      workflow.display_name.clone(),
    );
    attributes.insert(
      "pluginRunnerConnectorWorkflowAction".to_string(),
      workflow.action.clone(),
    );
    attributes.insert(
      "pluginRunnerConnectorWorkflowService".to_string(),
      workflow.service.clone(),
    );
    attributes.insert(
      "pluginRunnerConnectorWorkflowStages".to_string(),
      workflow.stages.join(", "),
    );
    attributes.insert(
      "pluginRunnerConnectorWorkflowStatuses".to_string(),
      workflow.statuses.join(", "),
    );
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
