use std::collections::HashMap;

use pith_plugin_host::{build_command_registry, PluginCommandEntry as HostPluginCommandEntry};
use pith_protocol::{
  JsonRpcRequest, JsonRpcResponse, PluginCommandRunParams, TimelineItem, WorkspaceSummary,
};

use super::plugin_command_builtins::is_supported_builtin_execution;
use super::plugin_command_types::{PluginCommandSnapshot, PreparedPluginCommandRun};
use crate::context_compaction::{merge_context_pack_attributes, pack_memory_context, ContextPack};
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
