use pith_model_runtime::GenerationCancellation;
use pith_plugin_host::{build_command_registry, PluginCommandEntry as HostPluginCommandEntry};
use pith_protocol::{JsonRpcRequest, JsonRpcResponse, PluginCommandRunParams, WorkspaceSummary};

use super::plugin_command_approval::{
  plugin_command_requires_user_approval, PLUGIN_COMMAND_APPROVAL_ACTION,
};
use super::plugin_command_readiness::command_readiness;
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

  let Some(command) = build_command_registry(context.plugin_state.catalog())
    .into_iter()
    .find(|command| command.command_id == params.command_id)
  else {
    return Err(JsonRpcResponse::error(
      request.id,
      -32052,
      "Plugin command not found",
    ));
  };
  let readiness = command_readiness(&command, &context.plugin_state);
  if !readiness.is_ready() {
    let error_code = if readiness.run_status == "needsConnectorAuth" {
      -32058
    } else {
      -32053
    };
    return Err(JsonRpcResponse::error(
      request.id,
      error_code,
      readiness.run_blocker.unwrap_or_else(|| {
        format!(
          "Plugin command `{}` is not ready to run.",
          command.command_id
        )
      }),
    ));
  }

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
  let connector_refs = build_command_connector_refs(&command, &context.plugin_state);
  let approval_id = plugin_command_requires_user_approval(&command, &connector_refs)
    .then(|| context.sequence_state.next_approval_id());
  let cancellation = GenerationCancellation::new();
  if context
    .execution_state
    .take_pending_running_cancel(&params.thread_id)
  {
    cancellation.cancel();
  }
  let running_id = format!("{}::{}", params.thread_id, command.command_id);
  context.execution_state.insert_running_plugin_command(
    running_id.clone(),
    params.thread_id.clone(),
    cancellation.clone(),
  );
  let snapshot = build_plugin_command_snapshot(
    context,
    params.thread_id,
    command,
    workspace,
    input,
    connector_refs,
    cancellation,
    running_id,
    approval_id,
  );

  Ok(PreparedPluginCommandRun {
    request_id: request.id,
    snapshot,
  })
}

pub(crate) fn prepare_approved_plugin_command_snapshot(
  context: &RuntimeContext,
  approval: &PendingApproval,
  workspace: Option<WorkspaceSummary>,
  cancellation: GenerationCancellation,
) -> std::result::Result<Option<PluginCommandSnapshot>, (i32, String)> {
  if approval.action != PLUGIN_COMMAND_APPROVAL_ACTION {
    return Ok(None);
  }
  let Some(command_id) = approval.command.as_deref() else {
    return Err((
      -32053,
      "Plugin command approval is missing its command id".to_string(),
    ));
  };
  let Some(command) = build_command_registry(context.plugin_state.catalog())
    .into_iter()
    .find(|command| command.command_id == command_id)
  else {
    return Err((-32052, "Plugin command not found".to_string()));
  };
  let readiness = command_readiness(&command, &context.plugin_state);
  if !readiness.is_ready() {
    return Err((
      -32053,
      readiness.run_blocker.unwrap_or_else(|| {
        format!(
          "Plugin command `{}` is not ready to run.",
          command.command_id
        )
      }),
    ));
  }

  let input = approval
    .content
    .as_deref()
    .map(str::trim)
    .filter(|input| !input.is_empty())
    .map(str::to_string);
  let connector_refs = build_command_connector_refs(&command, &context.plugin_state);
  let running_id = format!("{}::{}", approval.thread_id, command.command_id);

  Ok(Some(build_plugin_command_snapshot(
    context,
    approval.thread_id.clone(),
    command,
    workspace,
    input,
    connector_refs,
    cancellation,
    running_id,
    None,
  )))
}

fn build_plugin_command_snapshot(
  context: &RuntimeContext,
  thread_id: String,
  command: HostPluginCommandEntry,
  workspace: Option<WorkspaceSummary>,
  input: Option<String>,
  connector_refs: Vec<PluginConnectorExecutionRef>,
  cancellation: GenerationCancellation,
  running_id: String,
  approval_id: Option<String>,
) -> PluginCommandSnapshot {
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
