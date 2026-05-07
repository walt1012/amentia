use std::collections::HashMap;

use pith_protocol::{ApprovalRespondResult, JsonRpcResponse, TimelineItem};

use crate::approval_state::approvals_for_thread;
use crate::plugin_hooks::capture_plugin_hook_memory;
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
  } = completed.output;

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
