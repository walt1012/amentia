use std::collections::HashMap;

use pith_plugin_host::{build_command_registry, PluginCommandEntry as HostPluginCommandEntry};
use pith_protocol::{
  JsonRpcRequest, JsonRpcResponse, PluginCommandRunParams, TimelineItem, TurnStartResult,
  WorkspaceSummary,
};

use super::plugin_command_builtins::{
  execute_builtin_plugin_command, is_supported_builtin_execution,
};
use super::plugin_command_memory::{
  build_plugin_command_memory_warning_item, maybe_capture_plugin_command_memory,
};
use super::plugin_command_types::{
  CompletedPluginCommandRun, PluginCommandOutput, PluginCommandSnapshot, PreparedPluginCommandRun,
};
use crate::approval_state::approvals_for_thread;
use crate::context_compaction::{merge_context_pack_attributes, pack_memory_context, ContextPack};
use crate::request_params::parse_required_params;
use crate::thread_summary::refresh_thread_summary_note;
use crate::RuntimeContext;

pub(crate) fn handle_plugin_command_run(
  context: &mut RuntimeContext,
  request: JsonRpcRequest,
) -> JsonRpcResponse {
  let prepared = match prepare_plugin_command_run(context, request) {
    Ok(prepared) => prepared,
    Err(response) => return response,
  };
  let completed = execute_prepared_plugin_command_run(prepared);
  complete_prepared_plugin_command_run(context, completed)
}

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
  if !is_supported_builtin_execution(command.execution_kind.as_deref()) {
    return Err(JsonRpcResponse::error(
      request.id,
      -32053,
      format!(
        "Plugin command `{}` requires an explicit execution contract.",
        command.command_id
      ),
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
  let context_pack = pack_memory_context(
    context.model_state.runtime(),
    &memory_notes,
    workspace.as_ref().map(|entry| entry.display_name.as_str()),
    &memory_query,
  );
  let command_item = build_plugin_command_timeline_item(
    &command,
    workspace.as_ref(),
    input.as_deref(),
    &context_pack,
  );

  Ok(PreparedPluginCommandRun {
    request_id: request.id,
    snapshot: PluginCommandSnapshot {
      thread_id: params.thread_id,
      command,
      workspace,
      input,
      command_item,
      memory_notes,
    },
  })
}

pub fn execute_prepared_plugin_command_run(
  prepared: PreparedPluginCommandRun,
) -> CompletedPluginCommandRun {
  CompletedPluginCommandRun {
    request_id: prepared.request_id,
    output: execute_plugin_command_snapshot(prepared.snapshot),
  }
}

pub fn complete_prepared_plugin_command_run(
  context: &mut RuntimeContext,
  completed: CompletedPluginCommandRun,
) -> JsonRpcResponse {
  match completed.output {
    Ok(output) => match complete_plugin_command_items(context, output) {
      Ok(result) => JsonRpcResponse::success(completed.request_id, &result),
      Err((code, message)) => JsonRpcResponse::error(completed.request_id, code, message),
    },
    Err((code, message)) => JsonRpcResponse::error(completed.request_id, code, message),
  }
}

fn build_plugin_command_timeline_item(
  command: &HostPluginCommandEntry,
  workspace: Option<&WorkspaceSummary>,
  input: Option<&str>,
  context_pack: &ContextPack,
) -> TimelineItem {
  let mut attributes = HashMap::from([
    ("commandId".to_string(), command.command_id.clone()),
    ("pluginId".to_string(), command.plugin_id.clone()),
    (
      "pluginDisplayName".to_string(),
      command.plugin_display_name.clone(),
    ),
    ("sourcePath".to_string(), command.source_path.clone()),
  ]);
  if let Some(workspace) = workspace {
    attributes.insert(
      "workspaceDisplayName".to_string(),
      workspace.display_name.clone(),
    );
  }
  if let Some(input) = input {
    attributes.insert("commandInput".to_string(), input.to_string());
  }
  if let Some(execution_kind) = command.execution_kind.as_ref() {
    attributes.insert("executionKind".to_string(), execution_kind.clone());
  }
  merge_context_pack_attributes(&mut attributes, context_pack);

  let workspace_label = workspace
    .map(|entry| entry.display_name.clone())
    .unwrap_or_else(|| "No Workspace".to_string());
  let mut content = format!(
    "Run {} from {} in {}.\n{}",
    command.title, command.plugin_display_name, workspace_label, command.description
  );
  if let Some(input) = input {
    content.push_str(&format!("\nCommand input: {input}"));
  }

  TimelineItem {
    kind: "pluginCommand".to_string(),
    title: command.title.clone(),
    content,
    attributes: Some(attributes),
  }
}

fn execute_plugin_command_snapshot(
  snapshot: PluginCommandSnapshot,
) -> std::result::Result<PluginCommandOutput, (i32, String)> {
  let builtin_result = execute_builtin_plugin_command(
    &snapshot.command,
    snapshot.workspace.as_ref(),
    snapshot.input.as_deref(),
    &snapshot.memory_notes,
  )?;

  let result_item = build_plugin_result_timeline_item(
    &snapshot.command,
    &builtin_result.execution_kind,
    builtin_result.content.clone(),
  );
  let assistant_item = TimelineItem {
    kind: "assistantMessage".to_string(),
    title: "Assistant".to_string(),
    content: format!(
      "{} completed through {}.\n\n{}",
      snapshot.command.title, snapshot.command.plugin_display_name, builtin_result.content
    ),
    attributes: Some(HashMap::from([
      ("pluginId".to_string(), snapshot.command.plugin_id.clone()),
      ("commandId".to_string(), snapshot.command.command_id.clone()),
      (
        "executionKind".to_string(),
        builtin_result.execution_kind.clone(),
      ),
    ])),
  };
  Ok(PluginCommandOutput {
    thread_id: snapshot.thread_id,
    command: snapshot.command,
    workspace: snapshot.workspace,
    input: snapshot.input,
    items: vec![snapshot.command_item, result_item, assistant_item],
  })
}

fn complete_plugin_command_items(
  context: &mut RuntimeContext,
  output: PluginCommandOutput,
) -> std::result::Result<TurnStartResult, (i32, String)> {
  let PluginCommandOutput {
    thread_id: requested_thread_id,
    command,
    workspace,
    input,
    mut items,
  } = output;
  let prepared_thread = {
    let Some(thread) = context.thread_state.find_mut(&requested_thread_id) else {
      return Err((-32004, "Thread not found".to_string()));
    };

    let prepared_thread = thread.begin_plugin_command(workspace.clone());
    thread.append_items(items.clone());
    thread.mark_ready();
    prepared_thread
  };
  let thread_id = prepared_thread.thread_id;
  let turn_id = prepared_thread.turn_id;

  context
    .persist_runtime_state()
    .map_err(|error| (-32010, error.to_string()))?;
  refresh_thread_summary_note(context, &thread_id).map_err(|error| (-32012, error.to_string()))?;

  match maybe_capture_plugin_command_memory(
    context,
    &thread_id,
    &command,
    input.as_deref(),
    workspace.as_ref(),
    &items,
  ) {
    Ok(Some(memory_item)) => {
      if let Some(thread) = context.thread_state.find_mut(&thread_id) {
        thread.push_item(memory_item.clone());
      }
      items.push(memory_item);
      context
        .persist_runtime_state()
        .map_err(|error| (-32010, error.to_string()))?;
      refresh_thread_summary_note(context, &thread_id)
        .map_err(|error| (-32012, error.to_string()))?;
    }
    Ok(None) => {}
    Err(error) => {
      let warning_item = build_plugin_command_memory_warning_item(&command, error.to_string());
      if let Some(thread) = context.thread_state.find_mut(&thread_id) {
        thread.push_item(warning_item.clone());
      }
      items.push(warning_item);
      context
        .persist_runtime_state()
        .map_err(|error| (-32010, error.to_string()))?;
    }
  }

  Ok(TurnStartResult {
    turn_id,
    thread_id,
    items,
    pending_approvals: approvals_for_thread(context, &requested_thread_id),
    active_turn_id: None,
  })
}

fn build_plugin_result_timeline_item(
  command: &HostPluginCommandEntry,
  execution_kind: &str,
  content: String,
) -> TimelineItem {
  TimelineItem {
    kind: "pluginResult".to_string(),
    title: format!("{} Result", command.title),
    content,
    attributes: Some(HashMap::from([
      ("pluginId".to_string(), command.plugin_id.clone()),
      ("commandId".to_string(), command.command_id.clone()),
      ("executionKind".to_string(), execution_kind.to_string()),
      ("sourcePath".to_string(), command.source_path.clone()),
    ])),
  }
}
