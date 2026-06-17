use std::collections::HashMap;

use amentia_plugin_host::PluginCommandEntry as HostPluginCommandEntry;
use amentia_protocol::TimelineItem;

use super::plugin_command_approval::build_plugin_command_approval_request;
use super::plugin_command_builtins::execute_builtin_plugin_command;
use super::plugin_command_builtins::is_supported_builtin_execution;
use super::plugin_command_permission_gate::plugin_command_permission_denied_items;
use super::plugin_command_runner::{
  is_supported_external_plugin_execution, run_external_plugin_command,
};
use super::plugin_command_timeline::{
  build_plugin_assistant_timeline_item, build_plugin_failure_timeline_item,
  build_plugin_result_timeline_item, PluginFailureTimelineRequest,
};
use super::plugin_command_types::{
  CompletedPluginCommandRun, PluginCommandOutput, PluginCommandSnapshot, PreparedPluginCommandRun,
};

pub fn execute_prepared_plugin_command_run(
  prepared: PreparedPluginCommandRun,
) -> CompletedPluginCommandRun {
  let running_id = prepared.snapshot.running_id.clone();
  CompletedPluginCommandRun {
    request_id: prepared.request_id,
    running_id,
    output: execute_plugin_command_snapshot(prepared.snapshot),
  }
}

pub(crate) fn is_supported_plugin_command_execution(command: &HostPluginCommandEntry) -> bool {
  is_supported_builtin_execution(command.execution_kind.as_deref())
    || is_supported_external_plugin_execution(command)
}

pub(crate) fn execute_plugin_command_snapshot(
  snapshot: PluginCommandSnapshot,
) -> std::result::Result<PluginCommandOutput, (i32, String)> {
  let running_id = snapshot.running_id.clone();
  if snapshot.cancellation.is_cancelled() {
    return Ok(cancelled_plugin_command_output(snapshot, &running_id));
  }

  let (execution_kind, content, runner_memory_notes, attributes) =
    if is_supported_builtin_execution(snapshot.command.execution_kind.as_deref()) {
      let builtin_result = execute_builtin_plugin_command(
        &snapshot.command,
        snapshot.workspace.as_ref(),
        snapshot.input.as_deref(),
        &snapshot.memory_notes,
      )?;
      (
        builtin_result.execution_kind,
        builtin_result.content,
        vec![],
        builtin_result.attributes,
      )
    } else {
      if let Some(permission_items) = plugin_command_permission_denied_items(
        &snapshot.command,
        snapshot.workspace.as_ref(),
        &snapshot.connector_refs,
      ) {
        let mut items = vec![snapshot.command_item];
        items.extend(permission_items);
        tag_plugin_command_items(&mut items, &running_id);
        return Ok(PluginCommandOutput {
          thread_id: snapshot.thread_id,
          command: snapshot.command,
          workspace: snapshot.workspace,
          input: snapshot.input,
          items,
          capture_memory: false,
          runner_memory_notes: vec![],
          pending_approval: None,
        });
      }

      if let Some(approval_id) = snapshot.approval_id.clone() {
        let (approval, approval_items) = build_plugin_command_approval_request(
          approval_id,
          &snapshot.thread_id,
          &snapshot.command,
          snapshot.workspace.as_ref(),
          snapshot.input.as_deref(),
          &snapshot.connector_refs,
        );
        let mut items = vec![snapshot.command_item];
        items.extend(approval_items);
        tag_plugin_command_items(&mut items, &running_id);
        return Ok(PluginCommandOutput {
          thread_id: snapshot.thread_id,
          command: snapshot.command,
          workspace: snapshot.workspace,
          input: snapshot.input,
          items,
          capture_memory: false,
          runner_memory_notes: vec![],
          pending_approval: Some(approval),
        });
      }

      let runner_result = match run_external_plugin_command(
        &snapshot.command,
        &snapshot.thread_id,
        snapshot.workspace.as_ref(),
        snapshot.input.as_deref(),
        &snapshot.connector_refs,
        &snapshot.cancellation,
      ) {
        Ok(result) => result,
        Err(failure) => {
          let failure = *failure;
          let code = failure.code;
          let message = failure.message;
          let stdout = failure.stdout;
          let stderr = failure.stderr;
          let attributes = failure.attributes;
          let failure_item = build_plugin_failure_timeline_item(PluginFailureTimelineRequest {
            command: &snapshot.command,
            execution_kind: snapshot.command.execution_kind.as_deref(),
            code,
            message,
            input: snapshot.input.as_deref(),
            connector_refs: &snapshot.connector_refs,
            stdout: &stdout,
            stderr: &stderr,
            attributes,
          });
          let mut items = vec![snapshot.command_item, failure_item];
          tag_plugin_command_items(&mut items, &running_id);
          return Ok(PluginCommandOutput {
            thread_id: snapshot.thread_id,
            command: snapshot.command,
            workspace: snapshot.workspace,
            input: snapshot.input,
            items,
            capture_memory: false,
            runner_memory_notes: vec![],
            pending_approval: None,
          });
        }
      };
      if !runner_result.items.is_empty() {
        let runner_memory_notes = runner_result.memory_notes;
        let mut items = vec![snapshot.command_item];
        items.extend(runner_result.items);
        tag_plugin_command_items(&mut items, &running_id);
        return Ok(PluginCommandOutput {
          thread_id: snapshot.thread_id,
          command: snapshot.command,
          workspace: snapshot.workspace,
          input: snapshot.input,
          items,
          capture_memory: true,
          runner_memory_notes,
          pending_approval: None,
        });
      }
      (
        runner_result.execution_kind,
        runner_result.content,
        runner_result.memory_notes,
        runner_result.attributes,
      )
    };

  let mut result_item =
    build_plugin_result_timeline_item(&snapshot.command, &execution_kind, content.clone());
  if let Some(item_attributes) = result_item.attributes.as_mut() {
    item_attributes.extend(attributes);
  }
  let assistant_item =
    build_plugin_assistant_timeline_item(&snapshot.command, &execution_kind, &content);
  let mut items = vec![snapshot.command_item, result_item, assistant_item];
  tag_plugin_command_items(&mut items, &running_id);
  Ok(PluginCommandOutput {
    thread_id: snapshot.thread_id,
    command: snapshot.command,
    workspace: snapshot.workspace,
    input: snapshot.input,
    items,
    capture_memory: true,
    runner_memory_notes,
    pending_approval: None,
  })
}

fn cancelled_plugin_command_output(
  snapshot: PluginCommandSnapshot,
  running_id: &str,
) -> PluginCommandOutput {
  let failure_item = build_plugin_failure_timeline_item(PluginFailureTimelineRequest {
    command: &snapshot.command,
    execution_kind: snapshot.command.execution_kind.as_deref(),
    code: -32055,
    message: format!(
      "Plugin command `{}` was cancelled.",
      snapshot.command.command_id
    ),
    input: snapshot.input.as_deref(),
    connector_refs: &snapshot.connector_refs,
    stdout: "",
    stderr: "",
    attributes: HashMap::new(),
  });
  let mut items = vec![snapshot.command_item, failure_item];
  tag_plugin_command_items(&mut items, running_id);

  PluginCommandOutput {
    thread_id: snapshot.thread_id,
    command: snapshot.command,
    workspace: snapshot.workspace,
    input: snapshot.input,
    items,
    capture_memory: false,
    runner_memory_notes: vec![],
    pending_approval: None,
  }
}

fn tag_plugin_command_items(items: &mut [TimelineItem], running_id: &str) {
  for item in items {
    item
      .attributes
      .get_or_insert_with(HashMap::new)
      .insert("pluginCommandRunId".to_string(), running_id.to_string());
  }
}
