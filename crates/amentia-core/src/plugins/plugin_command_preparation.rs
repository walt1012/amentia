use std::collections::HashMap;

use amentia_memory::MemoryNote;
use amentia_model_runtime::{GenerationCancellation, LocalModelRuntime};
use amentia_plugin_host::{build_command_registry, PluginCommandEntry as HostPluginCommandEntry};
use amentia_protocol::{JsonRpcRequest, JsonRpcResponse, PluginCommandRunParams, WorkspaceSummary};

use super::plugin_command_approval::{
  plugin_command_requires_user_approval, PLUGIN_COMMAND_APPROVAL_ACTION,
};
use super::plugin_command_artifacts::expand_connector_saved_artifact_input;
use super::plugin_command_input_contract::validate_plugin_command_input_contract;
use super::plugin_command_preparation_error::PluginCommandPreparationError;
use super::plugin_command_readiness::command_readiness;
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
        "Session not found",
        "Select or create a session, then run the plugin action again.",
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
    HashMap::new(),
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
  planning_attributes: HashMap<String, String>,
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
    planning_attributes,
    cancellation,
    running_id,
  )
}

pub(crate) struct PluginCommandFollowUpRequest<'a> {
  pub(crate) plugin_state: &'a RuntimePluginState,
  pub(crate) model_runtime: &'a LocalModelRuntime,
  pub(crate) memory_notes: &'a [MemoryNote],
  pub(crate) thread_id: &'a str,
  pub(crate) workspace: Option<WorkspaceSummary>,
  pub(crate) command_id: &'a str,
  pub(crate) input: Option<String>,
  pub(crate) cancellation: GenerationCancellation,
  pub(crate) approval_id: Option<String>,
}

pub(crate) fn prepare_plugin_command_follow_up_snapshot(
  request: PluginCommandFollowUpRequest<'_>,
) -> std::result::Result<PluginCommandSnapshot, PluginCommandPreparationError> {
  let PluginCommandFollowUpRequest {
    plugin_state,
    model_runtime,
    memory_notes,
    thread_id,
    workspace,
    command_id,
    input,
    cancellation,
    approval_id,
  } = request;
  let command = resolve_plugin_command_from_state(plugin_state, command_id)?;
  let readiness = command_readiness(&command, plugin_state);
  if !readiness.is_ready() {
    return Err(PluginCommandPreparationError::from_readiness(
      &command, readiness,
    ));
  }

  let connector_refs = build_plugin_command_connector_refs(&command, plugin_state)?;
  let input = expand_connector_saved_artifact_input(workspace.as_ref(), input, &connector_refs);
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
        "Retry the request so Amentia can prepare a fresh bounded agent loop.",
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
      planning_attributes: HashMap::new(),
      connector_refs,
      cancellation,
      running_id,
      approval_id,
    },
  ))
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
  let connector_refs = build_plugin_command_connector_refs(&command, &context.plugin_state)?;
  let input = expand_connector_saved_artifact_input(workspace.as_ref(), input, &connector_refs);
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
      planning_attributes: HashMap::new(),
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
  planning_attributes: HashMap<String, String>,
  cancellation: GenerationCancellation,
  running_id: String,
) -> std::result::Result<PluginCommandSnapshot, PluginCommandPreparationError> {
  let readiness = command_readiness(&command, &context.plugin_state);
  if !readiness.is_ready() {
    return Err(PluginCommandPreparationError::from_readiness(
      &command, readiness,
    ));
  }

  let connector_refs = build_plugin_command_connector_refs(&command, &context.plugin_state)?;
  let input = expand_connector_saved_artifact_input(workspace.as_ref(), input, &connector_refs);
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
      planning_attributes,
      connector_refs,
      cancellation,
      running_id,
      approval_id,
    },
  ))
}

fn build_plugin_command_connector_refs(
  command: &HostPluginCommandEntry,
  plugin_state: &RuntimePluginState,
) -> std::result::Result<Vec<PluginConnectorExecutionRef>, PluginCommandPreparationError> {
  build_command_connector_refs(command, plugin_state)
    .map_err(|error| PluginCommandPreparationError::from_connector_execution_refs(command, error))
}

struct PluginCommandSnapshotDraft {
  thread_id: String,
  command: HostPluginCommandEntry,
  workspace: Option<WorkspaceSummary>,
  input: Option<String>,
  planning_attributes: HashMap<String, String>,
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
    planning_attributes,
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
    &planning_attributes,
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
