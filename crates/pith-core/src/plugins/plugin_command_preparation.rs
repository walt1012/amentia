use std::collections::HashMap;

use pith_model_runtime::GenerationCancellation;
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
use crate::RuntimeContext;

pub fn prepare_plugin_command_run(
  context: &mut RuntimeContext,
  request: JsonRpcRequest,
) -> std::result::Result<PreparedPluginCommandRun, JsonRpcResponse> {
  let params = parse_required_params::<PluginCommandRunParams>(&request, "plugin/commandRun")?;

  let Some(thread) = context.thread_state.find(&params.thread_id) else {
    return Err(JsonRpcResponse::error(
      request.id,
      -32004,
      "Thread not found",
    ));
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
  let command = resolve_plugin_command(context, &params.command_id)
    .map_err(|error| error.into_response(request.id.clone()))?;
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
    Self {
      code,
      message,
      data: Some(json!({
        "pluginId": &command.plugin_id,
        "commandId": &command.command_id,
        "runStatus": readiness.run_status,
        "runBlocker": readiness.run_blocker,
        "runRepairHint": readiness.run_repair_hint,
      })),
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
      data: Some(json!({
        "pluginId": &command.plugin_id,
        "commandId": &command.command_id,
        "runStatus": error.run_status,
        "runBlocker": message,
        "runRepairHint": error.run_repair_hint,
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
  ) -> HashMap<String, String> {
    let mut attributes = HashMap::from([
      ("pluginCommandRouting".to_string(), routing_reason.to_string()),
      ("commandId".to_string(), command_id.to_string()),
      ("errorCode".to_string(), self.code.to_string()),
    ]);
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

fn resolve_plugin_command(
  context: &RuntimeContext,
  command_id: &str,
) -> std::result::Result<HostPluginCommandEntry, PluginCommandPreparationError> {
  build_command_registry(context.plugin_state.catalog())
    .into_iter()
    .find(|command| command.command_id == command_id)
    .ok_or_else(|| PluginCommandPreparationError::plain(-32052, "Plugin command not found"))
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
    return Err(PluginCommandPreparationError::plain(
      -32052,
      "Plugin command not found",
    ));
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

  for field in execution.input.fields.iter().filter(|field| field.required) {
    match field.name.trim() {
      "threadId" | "commandId" | "envelope" => {}
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
  let memory_notes = context.memory_state.snapshot_notes();
  let memory_context = pack_memory_notes_for_context(
    context.model_state.runtime(),
    &memory_notes,
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
    memory_notes,
    cancellation,
    running_id,
    approval_id,
  }
}
