use std::collections::HashMap;

use pith_protocol::{ApprovalRespondResult, JsonRpcResponse, TimelineItem};

use crate::approval_state::approvals_for_thread;
use crate::plugin_commands::PluginCommandOutput;
use crate::plugin_hooks::capture_plugin_hook_memory;
use crate::plugins::plugin_command_memory::capture_plugin_command_output_memory;
use crate::request_state::{ApprovalExecutionOutput, CompletedApprovalRespond};
use crate::runtime_context::RuntimeContext;
use crate::thread_summary::refresh_thread_summary_note;

pub fn complete_prepared_approval_respond(
  context: &mut RuntimeContext,
  completed: CompletedApprovalRespond,
) -> JsonRpcResponse {
  let ApprovalExecutionOutput {
    approval,
    decision,
    workspace,
    mut items,
    memory_event,
    hook_memory_captures,
    approved_plugin_command_output,
  } = completed.output;
  context
    .execution_state
    .remove_running_approval(&approval.id);

  let Some(thread) = context.thread_state.find_mut(&approval.thread_id) else {
    return JsonRpcResponse::error(completed.request_id, -32004, "Thread not found");
  };
  thread.append_items(items.clone());
  thread.mark_ready();

  if let Err(error) = context.persist_resolved_approval(&approval, &decision) {
    return JsonRpcResponse::error(completed.request_id, -32010, error.to_string());
  }

  if let Err(error) = context.persist_runtime_state() {
    return JsonRpcResponse::error(completed.request_id, -32010, error.to_string());
  }

  if let Some(memory_event) = memory_event {
    if let Err(error) = context.remember(memory_event) {
      return JsonRpcResponse::error(completed.request_id, -32012, error.to_string());
    }
  }

  if !hook_memory_captures.is_empty() {
    let mut hook_memory_items = vec![];
    for capture in &hook_memory_captures {
      match capture_plugin_hook_memory(context, &workspace, capture) {
        Ok(item) => hook_memory_items.push(item),
        Err(error) => hook_memory_items.push(TimelineItem {
          kind: "warning".to_string(),
          title: "Hook Memory Capture Failed".to_string(),
          content: format!(
            "{} could not save its workspace memory note. {}",
            capture.hook.title, error
          ),
          attributes: Some(HashMap::from([
            ("pluginId".to_string(), capture.hook.plugin_id.clone()),
            ("hookId".to_string(), capture.hook.hook_id.clone()),
          ])),
        }),
      }
    }
    if let Some(thread) = context.thread_state.find_mut(&approval.thread_id) {
      thread.append_items(hook_memory_items.clone());
    }
    items.extend(hook_memory_items);

    if let Err(error) = context.persist_runtime_state() {
      return JsonRpcResponse::error(completed.request_id, -32010, error.to_string());
    }
  }

  if let Some(plugin_output) = approved_plugin_command_output {
    let plugin_memory_items = complete_approved_plugin_command_memory(context, plugin_output);
    if !plugin_memory_items.is_empty() {
      if let Some(thread) = context.thread_state.find_mut(&approval.thread_id) {
        thread.append_items(plugin_memory_items.clone());
      }
      items.extend(plugin_memory_items);

      if let Err(error) = context.persist_runtime_state() {
        return JsonRpcResponse::error(completed.request_id, -32010, error.to_string());
      }
    }
  }

  if let Err(error) = refresh_thread_summary_note(context, &approval.thread_id) {
    return JsonRpcResponse::error(completed.request_id, -32012, error.to_string());
  }

  let pending_approvals = approvals_for_thread(context, &approval.thread_id);

  JsonRpcResponse::success(
    completed.request_id,
    &ApprovalRespondResult {
      approval_id: approval.id,
      thread_id: approval.thread_id.clone(),
      items,
      pending_approvals,
    },
  )
}

fn complete_approved_plugin_command_memory(
  context: &mut RuntimeContext,
  output: PluginCommandOutput,
) -> Vec<TimelineItem> {
  capture_plugin_command_output_memory(
    context,
    &output.thread_id,
    &output.command,
    output.workspace.as_ref(),
    output.input.as_deref(),
    &output.items,
    output.capture_memory,
    &output.runner_memory_notes,
  )
}
