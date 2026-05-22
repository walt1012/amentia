use std::collections::HashMap;

use pith_memory::MemoryNote;
use pith_model_runtime::{GenerationCancellation, LocalModelRuntime};
use pith_plugin_host::{build_command_registry, PluginCommandEntry as HostPluginCommandEntry};
use pith_protocol::{JsonRpcRequest, JsonRpcResponse, PluginCommandRunParams, WorkspaceSummary};
use serde_json::{json, Value};

use super::plugin_command_approval::{
  plugin_command_requires_user_approval, PLUGIN_COMMAND_APPROVAL_ACTION,
};
use super::plugin_command_readiness::{command_readiness, PluginCommandReadiness};
use super::plugin_command_timeline::build_plugin_command_timeline_item;
use super::plugin_command_types::{
  PluginCommandSnapshot, PluginConnectorExecutionRef, PreparedPluginCommandRun,
};
use super::plugin_connector_execution_refs::build_command_connector_refs;
use crate::approval_types::PendingApproval;
use crate::context_memory_pack::pack_memory_notes_for_context;
use crate::request_params::parse_required_params;
use crate::runtime_plugins::RuntimePluginState;
use crate::RuntimeContext;

pub fn prepare_plugin_command_run(
  context: &mut RuntimeContext,
  request: JsonRpcRequest,
) -> std::result::Result<PreparedPluginCommandRun, JsonRpcResponse> {
  let params = parse_required_params::<PluginCommandRunParams>(&request, "plugin/commandRun")?;
  let command = resolve_plugin_command(context, &params.command_id)
    .map_err(|error| error.into_response(request.id.clone()))?;
  let readiness = command_readiness(&command, &context.plugin_state);
  if !readiness.is_ready() {
    return Err(
      PluginCommandPreparationError::from_readiness(&command, readiness).into_response(request.id),
    );
  }

  let Some(thread) = context.thread_state.find(&params.thread_id) else {
    return Err(
      PluginCommandPreparationError::from_command_context(
        &command,
        -32004,
        "missingThread",
        "Thread not found",
        "Select or create a thread, then run the plugin command again.",
      )
      .into_response(request.id),
    );
  };

  let workspace = thread
    .workspace_cloned()
    .or_else(|| context.workspace_state.current_cloned());
  let input = params
    .input
    .as_deref()
    .map(str::trim)
    .filter(|input| !input.is_empty())
    .map(str::to_string);
  let cancellation = GenerationCancellation::new();
  if context
    .execution_state
    .take_pending_running_cancel(&params.thread_id)
  {
    cancellation.cancel();
  }
  let running_id = format!("{}::{}", params.thread_id, command.command_id);
  let snapshot = prepare_plugin_command_snapshot_for_execution(
    context,
    params.thread_id.clone(),
    command,
    workspace,
    input,
    cancellation.clone(),
    running_id.clone(),
  )
  .map_err(|error| error.into_response(request.id.clone()))?;
  context.execution_state.insert_running_plugin_command(
    running_id.clone(),
    params.thread_id.clone(),
    cancellation.clone(),
  );

  Ok(PreparedPluginCommandRun {
    request_id: request.id,
    snapshot,
  })
}

pub(crate) fn prepare_plugin_command_turn_snapshot(
  context: &mut RuntimeContext,
  thread_id: &str,
  workspace: Option<WorkspaceSummary>,
  command_id: &str,
  input: Option<String>,
  cancellation: GenerationCancellation,
) -> std::result::Result<PluginCommandSnapshot, PluginCommandPreparationError> {
  let command = resolve_plugin_command(context, command_id)?;
  let running_id = format!("{thread_id}::{}", command.command_id);
  prepare_plugin_command_snapshot_for_execution(
    context,
    thread_id.to_string(),
    command,
    workspace,
    input,
    cancellation,
    running_id,
  )
}

pub(crate) fn prepare_plugin_command_follow_up_snapshot(
  plugin_state: &RuntimePluginState,
  model_runtime: &LocalModelRuntime,
  memory_notes: &[MemoryNote],
  thread_id: &str,
  workspace: Option<WorkspaceSummary>,
  command_id: &str,
  input: Option<String>,
  cancellation: GenerationCancellation,
  approval_id: Option<String>,
) -> std::result::Result<PluginCommandSnapshot, PluginCommandPreparationError> {
  let command = resolve_plugin_command_from_state(plugin_state, command_id)?;
  let readiness = command_readiness(&command, plugin_state);
  if !readiness.is_ready() {
    return Err(PluginCommandPreparationError::from_readiness(
      &command, readiness,
    ));
  }

  let connector_refs = build_command_connector_refs(&command, plugin_state);
  if let Err(error) = validate_plugin_command_input_contract(
    &command,
    workspace.as_ref(),
    input.as_deref(),
    &connector_refs,
  ) {
    return Err(PluginCommandPreparationError::from_input_contract(
      &command, error,
    ));
  }
  let approval_id = if plugin_command_requires_user_approval(&command, &connector_refs) {
    Some(approval_id.ok_or_else(|| {
      PluginCommandPreparationError::from_command_context(
        &command,
        -32053,
        "missingApprovalReservation",
        "Plugin command approval id was not reserved for this turn.",
        "Retry the request so Pith can prepare a fresh bounded agent loop.",
      )
    })?)
  } else {
    None
  };
  let running_id = format!("{thread_id}::{}", command.command_id);

  Ok(build_plugin_command_snapshot_from_parts(
    model_runtime,
    memory_notes,
    PluginCommandSnapshotDraft {
      thread_id: thread_id.to_string(),
      command,
      workspace,
      input,
      connector_refs,
      cancellation,
      running_id,
      approval_id,
    },
  ))
}

pub(crate) struct PluginCommandPreparationError {
  code: i32,
  message: String,
  data: Option<Value>,
}

impl PluginCommandPreparationError {
  fn plain(code: i32, message: impl Into<String>) -> Self {
    Self {
      code,
      message: message.into(),
      data: None,
    }
  }

  fn from_readiness(command: &HostPluginCommandEntry, readiness: PluginCommandReadiness) -> Self {
    let code = if readiness.run_status == "needsConnectorAuth" {
      -32058
    } else {
      -32053
    };
    let message = readiness.run_blocker.clone().unwrap_or_else(|| {
      format!(
        "Plugin command `{}` is not ready to run.",
        command.command_id
      )
    });
    let mut data = json!({
      "pluginId": &command.plugin_id,
      "commandId": &command.command_id,
      "sourcePath": &command.source_path,
      "runStatus": readiness.run_status,
      "runBlocker": readiness.run_blocker,
      "runRepairHint": readiness.run_repair_hint,
    });
    insert_connector_error_data(
      &mut data,
      readiness_connector_ids(&command.plugin_id, &readiness),
    );
    Self {
      code,
      message,
      data: Some(data),
    }
  }

  fn from_input_contract(
    command: &HostPluginCommandEntry,
    error: PluginCommandInputContractError,
  ) -> Self {
    let message = error.message;
    Self {
      code: -32053,
      message: message.clone(),
      data: Some({
        let mut data = json!({
          "pluginId": &command.plugin_id,
          "commandId": &command.command_id,
          "sourcePath": &command.source_path,
          "runStatus": error.run_status,
          "runBlocker": message,
          "runRepairHint": error.run_repair_hint,
        });
        insert_connector_error_data(&mut data, declared_connector_ids(command));
        data
      }),
    }
  }

  fn from_command_context(
    command: &HostPluginCommandEntry,
    code: i32,
    run_status: &'static str,
    message: impl Into<String>,
    run_repair_hint: &'static str,
  ) -> Self {
    let message = message.into();
    Self {
      code,
      message: message.clone(),
      data: Some(json!({
        "pluginId": &command.plugin_id,
        "commandId": &command.command_id,
        "sourcePath": &command.source_path,
        "runStatus": run_status,
        "runBlocker": message,
        "runRepairHint": run_repair_hint,
      })),
    }
  }

  fn command_not_found(command_id: &str) -> Self {
    let message = "Plugin command not found";
    let repair_hint = concat!(
      "Refresh plugins, enable the plugin, ",
      "or select a command from the command panel."
    );
    Self {
      code: -32052,
      message: message.to_string(),
      data: Some(json!({
        "commandId": command_id,
        "runStatus": "commandNotFound",
        "runBlocker": message,
        "runRepairHint": repair_hint,
      })),
    }
  }

  pub(crate) fn into_response(self, request_id: Value) -> JsonRpcResponse {
    if let Some(data) = self.data {
      JsonRpcResponse::error_with_data(request_id, self.code, self.message, &data)
    } else {
      JsonRpcResponse::error(request_id, self.code, self.message)
    }
  }

  pub(crate) fn message(&self) -> &str {
    &self.message
  }

  pub(crate) fn route_failure_attributes(
    &self,
    command_id: &str,
    routing_reason: &str,
    input: Option<&str>,
  ) -> HashMap<String, String> {
    let mut attributes = HashMap::from([
      ("pluginCommandStatus".to_string(), "blocked".to_string()),
      (
        "pluginCommandRouting".to_string(),
        routing_reason.to_string(),
      ),
      ("commandId".to_string(), command_id.to_string()),
      ("errorCode".to_string(), self.code.to_string()),
    ]);
    if let Some(input) = input.map(str::trim).filter(|input| !input.is_empty()) {
      attributes.insert("commandInput".to_string(), input.to_string());
    }
    if let Some(data) = self.data.as_ref().and_then(|data| data.as_object()) {
      for (key, value) in data {
        match value {
          Value::String(text) if !text.is_empty() => {
            attributes.insert(key.to_string(), text.to_string());
          }
          Value::Null => {}
          value => {
            attributes.insert(key.to_string(), value.to_string());
          }
        }
      }
    }
    attributes
  }
}

fn insert_connector_error_data(data: &mut Value, connector_ids: Vec<String>) {
  if connector_ids.is_empty() {
    return;
  }

  data["connectorIds"] = json!(connector_ids.join(", "));
  if connector_ids.len() == 1 {
    data["connectorId"] = json!(&connector_ids[0]);
  }
}

fn readiness_connector_ids(plugin_id: &str, readiness: &PluginCommandReadiness) -> Vec<String> {
  if readiness.required_connector_ids.is_empty() {
    return qualify_connector_ids(plugin_id, &readiness.declared_connector_ids);
  }

  qualify_connector_ids(plugin_id, &readiness.required_connector_ids)
}

fn declared_connector_ids(command: &HostPluginCommandEntry) -> Vec<String> {
  let Some(connector_ids) = command
    .execution
    .as_ref()
    .and_then(|execution| execution.connector_ids.as_ref())
  else {
    return vec![];
  };

  qualify_connector_ids(&command.plugin_id, connector_ids)
}

fn qualify_connector_ids(plugin_id: &str, connector_ids: &[String]) -> Vec<String> {
  connector_ids
    .iter()
    .map(|connector_id| {
      if connector_id.contains("::") {
        connector_id.clone()
      } else {
        format!("{plugin_id}::{connector_id}")
      }
    })
    .collect()
}

fn resolve_plugin_command(
  context: &RuntimeContext,
  command_id: &str,
) -> std::result::Result<HostPluginCommandEntry, PluginCommandPreparationError> {
  resolve_plugin_command_from_state(&context.plugin_state, command_id)
}

fn resolve_plugin_command_from_state(
  plugin_state: &RuntimePluginState,
  command_id: &str,
) -> std::result::Result<HostPluginCommandEntry, PluginCommandPreparationError> {
  build_command_registry(plugin_state.catalog())
    .into_iter()
    .find(|command| command.command_id == command_id)
    .ok_or_else(|| PluginCommandPreparationError::command_not_found(command_id))
}

pub(crate) fn prepare_approved_plugin_command_snapshot(
  context: &RuntimeContext,
  approval: &PendingApproval,
  workspace: Option<WorkspaceSummary>,
  cancellation: GenerationCancellation,
) -> std::result::Result<Option<PluginCommandSnapshot>, PluginCommandPreparationError> {
  if approval.action != PLUGIN_COMMAND_APPROVAL_ACTION {
    return Ok(None);
  }
  let Some(command_id) = approval.command.as_deref() else {
    return Err(PluginCommandPreparationError::plain(
      -32053,
      "Plugin command approval is missing its command id",
    ));
  };
  let Some(command) = build_command_registry(context.plugin_state.catalog())
    .into_iter()
    .find(|command| command.command_id == command_id)
  else {
    return Err(PluginCommandPreparationError::command_not_found(command_id));
  };
  let readiness = command_readiness(&command, &context.plugin_state);
  if !readiness.is_ready() {
    return Err(PluginCommandPreparationError::from_readiness(
      &command, readiness,
    ));
  }

  let input = approval
    .content
    .as_deref()
    .map(str::trim)
    .filter(|input| !input.is_empty())
    .map(str::to_string);
  let connector_refs = build_command_connector_refs(&command, &context.plugin_state);
  if let Err(error) = validate_plugin_command_input_contract(
    &command,
    workspace.as_ref(),
    input.as_deref(),
    &connector_refs,
  ) {
    return Err(PluginCommandPreparationError::from_input_contract(
      &command, error,
    ));
  }
  let running_id = format!("{}::{}", approval.thread_id, command.command_id);

  Ok(Some(build_plugin_command_snapshot(
    context,
    PluginCommandSnapshotDraft {
      thread_id: approval.thread_id.clone(),
      command,
      workspace,
      input,
      connector_refs,
      cancellation,
      running_id,
      approval_id: None,
    },
  )))
}

fn prepare_plugin_command_snapshot_for_execution(
  context: &mut RuntimeContext,
  thread_id: String,
  command: HostPluginCommandEntry,
  workspace: Option<WorkspaceSummary>,
  input: Option<String>,
  cancellation: GenerationCancellation,
  running_id: String,
) -> std::result::Result<PluginCommandSnapshot, PluginCommandPreparationError> {
  let readiness = command_readiness(&command, &context.plugin_state);
  if !readiness.is_ready() {
    return Err(PluginCommandPreparationError::from_readiness(
      &command, readiness,
    ));
  }

  let connector_refs = build_command_connector_refs(&command, &context.plugin_state);
  if let Err(error) = validate_plugin_command_input_contract(
    &command,
    workspace.as_ref(),
    input.as_deref(),
    &connector_refs,
  ) {
    return Err(PluginCommandPreparationError::from_input_contract(
      &command, error,
    ));
  }
  let approval_id = plugin_command_requires_user_approval(&command, &connector_refs)
    .then(|| context.sequence_state.next_approval_id());

  Ok(build_plugin_command_snapshot(
    context,
    PluginCommandSnapshotDraft {
      thread_id,
      command,
      workspace,
      input,
      connector_refs,
      cancellation,
      running_id,
      approval_id,
    },
  ))
}

fn validate_plugin_command_input_contract(
  command: &HostPluginCommandEntry,
  workspace: Option<&WorkspaceSummary>,
  input: Option<&str>,
  connector_refs: &[PluginConnectorExecutionRef],
) -> std::result::Result<(), PluginCommandInputContractError> {
  let Some(execution) = command.execution.as_ref() else {
    return Ok(());
  };

  if let Some(field) = input.and_then(|_| {
    execution
      .input
      .fields
      .iter()
      .find(|field| field.name.trim() == "input" && !input_field_accepts_plain_text(field))
  }) {
    return Err(PluginCommandInputContractError::new(
      format!(
        "Plugin command `{}` declares input field `input` with unsupported kind `{}` for plain command input.",
        command.command_id, field.kind
      ),
      "unsupportedInputField",
      "Use a text or string `input` field for plain input, or omit plain input until structured command inputs are supported.",
    ));
  }

  for field in execution.input.fields.iter().filter(|field| field.required) {
    match field.name.trim() {
      "threadId" | "commandId" | "envelope" => {}
      "input" if !input_field_accepts_plain_text(field) => {
        return Err(PluginCommandInputContractError::new(
          format!(
            "Plugin command `{}` requires unsupported input field `input` of kind `{}`.",
            command.command_id, field.kind
          ),
          "unsupportedInputField",
          "Use a text or string `input` field for plain input, or make the structured input optional.",
        ));
      }
      "input" if input.is_some() => {}
      "input" => {
        return Err(PluginCommandInputContractError::new(
          format!(
            "Plugin command `{}` requires command input field `input`.",
            command.command_id
          ),
          "missingInput",
          "Run the command with input or make the `input` field optional.",
        ));
      }
      "workspace" if workspace.is_some() => {}
      "workspace" => {
        return Err(PluginCommandInputContractError::new(
          format!(
            "Plugin command `{}` requires an open workspace for input field `workspace`.",
            command.command_id
          ),
          "missingWorkspaceInput",
          "Open a workspace before running this command or make the field optional.",
        ));
      }
      "connectors" if !connector_refs.is_empty() => {}
      "connectors" => {
        return Err(PluginCommandInputContractError::new(
          format!(
            "Plugin command `{}` requires connector input field `connectors`, but no connector context is available.",
            command.command_id
          ),
          "missingConnectorInput",
          "Declare and authorize a connector, or make the connector input optional.",
        ));
      }
      field_name => {
        return Err(PluginCommandInputContractError::new(
          format!(
            "Plugin command `{}` requires unsupported input field `{field_name}`.",
            command.command_id
          ),
          "unsupportedInputField",
          "Use supported required fields: threadId, commandId, envelope, input, workspace, or connectors.",
        ));
      }
    }
  }

  Ok(())
}

fn input_field_accepts_plain_text(
  field: &pith_plugin_host::PluginCommandEnvelopeFieldEntry,
) -> bool {
  matches!(
    field.kind.trim().to_ascii_lowercase().as_str(),
    "text" | "string"
  )
}

struct PluginCommandInputContractError {
  message: String,
  run_status: &'static str,
  run_repair_hint: &'static str,
}

impl PluginCommandInputContractError {
  fn new(message: String, run_status: &'static str, run_repair_hint: &'static str) -> Self {
    Self {
      message,
      run_status,
      run_repair_hint,
    }
  }
}

struct PluginCommandSnapshotDraft {
  thread_id: String,
  command: HostPluginCommandEntry,
  workspace: Option<WorkspaceSummary>,
  input: Option<String>,
  connector_refs: Vec<PluginConnectorExecutionRef>,
  cancellation: GenerationCancellation,
  running_id: String,
  approval_id: Option<String>,
}

fn build_plugin_command_snapshot(
  context: &RuntimeContext,
  draft: PluginCommandSnapshotDraft,
) -> PluginCommandSnapshot {
  let memory_notes = context.memory_state.snapshot_notes();
  build_plugin_command_snapshot_from_parts(context.model_state.runtime(), &memory_notes, draft)
}

fn build_plugin_command_snapshot_from_parts(
  model_runtime: &LocalModelRuntime,
  memory_notes: &[MemoryNote],
  draft: PluginCommandSnapshotDraft,
) -> PluginCommandSnapshot {
  let PluginCommandSnapshotDraft {
    thread_id,
    command,
    workspace,
    input,
    connector_refs,
    cancellation,
    running_id,
    approval_id,
  } = draft;
  let memory_query = input
    .as_deref()
    .map(|input| {
      format!(
        "{} {} {} {}",
        command.title, command.description, command.prompt, input
      )
    })
    .unwrap_or_else(|| {
      format!(
        "{} {} {}",
        command.title, command.description, command.prompt
      )
    });
  let memory_context = pack_memory_notes_for_context(
    model_runtime,
    memory_notes,
    workspace.as_ref().map(|entry| entry.display_name.as_str()),
    &memory_query,
  );
  let command_item = build_plugin_command_timeline_item(
    &command,
    workspace.as_ref(),
    input.as_deref(),
    &memory_context,
    &connector_refs,
  );

  PluginCommandSnapshot {
    thread_id,
    command,
    workspace,
    input,
    connector_refs,
    command_item,
    memory_notes: memory_notes.to_vec(),
    cancellation,
    running_id,
    approval_id,
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn route_failure_attributes_keep_source_path_and_input() {
    let command = HostPluginCommandEntry {
      command_id: "test-plugin::run".to_string(),
      title: "Run Test Plugin".to_string(),
      description: "Run a test plugin command.".to_string(),
      prompt: "Run the plugin.".to_string(),
      plugin_id: "test-plugin".to_string(),
      plugin_display_name: "Test Plugin".to_string(),
      permissions: vec![],
      source_path: "plugins/test-plugin/commands/run.json".to_string(),
      execution: None,
      execution_kind: Some("stdio.test".to_string()),
      manifest_error: None,
      memory_note_title: None,
      memory_note_source: None,
      memory_note_tags: vec![],
    };
    let error = PluginCommandPreparationError::from_input_contract(
      &command,
      PluginCommandInputContractError::new(
        "Missing input.".to_string(),
        "missingInput",
        "Run with input.",
      ),
    );

    let attributes = error.route_failure_attributes(
      "test-plugin::run",
      "slashPluginCommand",
      Some("retry this input"),
    );

    assert_eq!(
      attributes.get("sourcePath").map(String::as_str),
      Some("plugins/test-plugin/commands/run.json")
    );
    assert_eq!(
      attributes.get("pluginCommandStatus").map(String::as_str),
      Some("blocked")
    );
    assert_eq!(
      attributes.get("commandInput").map(String::as_str),
      Some("retry this input")
    );
    assert_eq!(
      attributes.get("runRepairHint").map(String::as_str),
      Some("Run with input.")
    );
  }

  #[test]
  fn input_contract_failure_keeps_connector_ids_for_repair() {
    let command = HostPluginCommandEntry {
      command_id: "test-plugin::run".to_string(),
      title: "Run Test Plugin".to_string(),
      description: "Run a test plugin command.".to_string(),
      prompt: "Run the plugin.".to_string(),
      plugin_id: "test-plugin".to_string(),
      plugin_display_name: "Test Plugin".to_string(),
      permissions: vec![],
      source_path: "plugins/test-plugin/commands/run.json".to_string(),
      execution: Some(test_connector_execution()),
      execution_kind: Some("stdio.test".to_string()),
      manifest_error: None,
      memory_note_title: None,
      memory_note_source: None,
      memory_note_tags: vec![],
    };
    let error = PluginCommandPreparationError::from_input_contract(
      &command,
      PluginCommandInputContractError::new(
        "Missing connector input.".to_string(),
        "missingConnectorInput",
        "Declare and authorize a connector.",
      ),
    );

    let data = error
      .data
      .as_ref()
      .and_then(|data| data.as_object())
      .expect("error data");
    assert_eq!(
      data.get("connectorId").and_then(Value::as_str),
      Some("test-plugin::notion")
    );
    assert_eq!(
      data.get("connectorIds").and_then(Value::as_str),
      Some("test-plugin::notion")
    );
  }

  #[test]
  fn input_contract_rejects_structured_required_input_field() {
    let command = test_command_with_execution(test_input_execution("object", true));

    let error =
      validate_plugin_command_input_contract(&command, None, None, &[]).expect_err("input error");

    assert_eq!(error.run_status, "unsupportedInputField");
    assert!(error.message.contains("unsupported input field `input`"));
    assert!(error.run_repair_hint.contains("text or string"));
  }

  #[test]
  fn input_contract_rejects_plain_input_for_structured_optional_field() {
    let command = test_command_with_execution(test_input_execution("json", false));

    let error = validate_plugin_command_input_contract(&command, None, Some("plain input"), &[])
      .expect_err("input error");

    assert_eq!(error.run_status, "unsupportedInputField");
    assert!(error.message.contains("unsupported kind `json`"));
    assert!(error.run_repair_hint.contains("structured command inputs"));
  }

  fn test_command_with_execution(
    execution: pith_plugin_host::PluginCommandExecutionEntry,
  ) -> HostPluginCommandEntry {
    HostPluginCommandEntry {
      command_id: "test-plugin::run".to_string(),
      title: "Run Test Plugin".to_string(),
      description: "Run a test plugin command.".to_string(),
      prompt: "Run the plugin.".to_string(),
      plugin_id: "test-plugin".to_string(),
      plugin_display_name: "Test Plugin".to_string(),
      permissions: vec![],
      source_path: "plugins/test-plugin/commands/run.json".to_string(),
      execution: Some(execution),
      execution_kind: Some("stdio.test".to_string()),
      manifest_error: None,
      memory_note_title: None,
      memory_note_source: None,
      memory_note_tags: vec![],
    }
  }

  fn test_input_execution(
    input_kind: &str,
    input_required: bool,
  ) -> pith_plugin_host::PluginCommandExecutionEntry {
    pith_plugin_host::PluginCommandExecutionEntry {
      kind: "stdio.test".to_string(),
      driver: "stdio".to_string(),
      entrypoint: Some("runner.sh".to_string()),
      connector_ids: None,
      input: pith_plugin_host::PluginCommandEnvelopeEntry {
        envelope: "pith.plugin.command.input".to_string(),
        fields: vec![pith_plugin_host::PluginCommandEnvelopeFieldEntry {
          name: "input".to_string(),
          kind: input_kind.to_string(),
          required: input_required,
          description: None,
        }],
      },
      output: pith_plugin_host::PluginCommandEnvelopeEntry {
        envelope: "pith.plugin.command.output".to_string(),
        fields: vec![],
      },
    }
  }

  fn test_connector_execution() -> pith_plugin_host::PluginCommandExecutionEntry {
    pith_plugin_host::PluginCommandExecutionEntry {
      kind: "stdio.test".to_string(),
      driver: "stdio".to_string(),
      entrypoint: Some("runner.sh".to_string()),
      connector_ids: Some(vec!["notion".to_string()]),
      input: pith_plugin_host::PluginCommandEnvelopeEntry {
        envelope: "pith.plugin.command.input".to_string(),
        fields: vec![],
      },
      output: pith_plugin_host::PluginCommandEnvelopeEntry {
        envelope: "pith.plugin.command.output".to_string(),
        fields: vec![],
      },
    }
  }
}
