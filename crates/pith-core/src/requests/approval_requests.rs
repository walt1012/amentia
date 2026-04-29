use std::collections::HashMap;
use std::path::Path;

use pith_memory::MemoryEvent;
use pith_protocol::{
  ApprovalRespondParams, ApprovalRespondResult, JsonRpcRequest, JsonRpcResponse, TimelineItem,
};
use pith_tools::{run_shell, write_file};

use crate::approval_state::approvals_for_thread;
use crate::local_responses::{
  format_shell_result, summarize_denied_approval, summarize_shell_result,
};
use crate::plugin_hooks::{build_shell_completed_hook_items, capture_plugin_hook_memory};
use crate::plugin_permissions::{
  build_permission_denied_items, granted_permission_sources, permission_is_granted,
};
use crate::request_params::parse_required_params;
use crate::request_state::{
  ApprovalExecutionOutput, CompletedApprovalRespond, PreparedApprovalRespond,
  PreparedApprovalSnapshot,
};
use crate::runtime_context::RuntimeContext;
use crate::thread_summary::refresh_thread_summary_note;

pub(crate) fn handle_approval_respond(
  context: &mut RuntimeContext,
  request: JsonRpcRequest,
) -> JsonRpcResponse {
  let prepared = match prepare_approval_respond(context, request) {
    Ok(prepared) => prepared,
    Err(response) => return response,
  };
  let completed = execute_prepared_approval_respond(prepared);
  complete_prepared_approval_respond(context, completed)
}

pub fn prepare_approval_respond(
  context: &mut RuntimeContext,
  request: JsonRpcRequest,
) -> std::result::Result<PreparedApprovalRespond, JsonRpcResponse> {
  let params = parse_required_params::<ApprovalRespondParams>(&request, "approval/respond")?;
  let decision = params.decision.to_lowercase();
  if decision != "approved" && decision != "denied" {
    return Err(JsonRpcResponse::error(
      request.id,
      -32602,
      "approval/respond decision must be approved or denied",
    ));
  }

  let Some(approval) = context
    .execution_state
    .pending_approval(&params.approval_id)
    .cloned()
  else {
    return Err(JsonRpcResponse::error(
      request.id,
      -32030,
      "Approval request not found",
    ));
  };
  let current_workspace = context.workspace_state.current.clone();
  let model_runtime = context.model_state.snapshot();
  let memory_notes = context.memory_state.notes().to_vec();
  let permission_sources = granted_permission_sources(&context.plugin_state.catalog);
  let plugins = context.plugin_state.catalog.clone();

  let Some(thread) = context
    .threads
    .iter_mut()
    .find(|thread| thread.summary.id == approval.thread_id)
  else {
    return Err(JsonRpcResponse::error(
      request.id,
      -32004,
      "Thread not found",
    ));
  };
  if thread.workspace.is_none() {
    thread.workspace = current_workspace;
    thread.summary.workspace = thread.workspace.clone();
  }
  let Some(workspace) = thread.workspace.clone() else {
    return Err(JsonRpcResponse::error(
      request.id,
      -32031,
      "Open a workspace for this thread before resolving approvals",
    ));
  };
  thread.summary.status = format!("Resolving approval {}", approval.id);
  context
    .execution_state
    .remove_pending_approval(&params.approval_id);

  Ok(PreparedApprovalRespond {
    request_id: request.id,
    snapshot: PreparedApprovalSnapshot {
      approval,
      decision,
      workspace,
      model_runtime,
      memory_notes,
      permission_sources,
      plugins,
    },
  })
}

pub fn execute_prepared_approval_respond(
  prepared: PreparedApprovalRespond,
) -> CompletedApprovalRespond {
  CompletedApprovalRespond {
    request_id: prepared.request_id,
    output: execute_approval_snapshot(prepared.snapshot),
  }
}

fn execute_approval_snapshot(snapshot: PreparedApprovalSnapshot) -> ApprovalExecutionOutput {
  let PreparedApprovalSnapshot {
    approval,
    decision,
    workspace,
    model_runtime,
    memory_notes,
    permission_sources,
    plugins,
  } = snapshot;
  let mut items = vec![];
  let mut memory_event = None;
  let mut hook_memory_captures = vec![];
  if decision == "approved" {
    items.push(TimelineItem {
      kind: "approvalResolved".to_string(),
      title: "Approval Granted".to_string(),
      content: format!(
        "Approved {} for {}.",
        approval.action, approval.relative_path
      ),
      attributes: Some(HashMap::from([
        ("approvalId".to_string(), approval.id.clone()),
        ("decision".to_string(), "approved".to_string()),
      ])),
    });
    if approval.action == "write_file" {
      if !permission_is_granted(&permission_sources, "file.write") {
        items.extend(build_permission_denied_items(
          &permission_sources,
          "file.write",
          "complete the approved file write",
          &workspace.display_name,
          HashMap::from([
            ("approvalId".to_string(), approval.id.clone()),
            ("relativePath".to_string(), approval.relative_path.clone()),
          ]),
        ));
      } else {
        let content = approval.content.clone().unwrap_or_default();
        items.push(TimelineItem {
          kind: "toolStart".to_string(),
          title: "write_file".to_string(),
          content: approval.relative_path.clone(),
          attributes: None,
        });

        match write_file(
          Path::new(&workspace.root_path),
          &approval.relative_path,
          &content,
        ) {
          Ok(relative_path) => {
            memory_event = Some(MemoryEvent::FileWritten {
              workspace_display_name: workspace.display_name.clone(),
              relative_path: relative_path.clone(),
            });
            items.push(TimelineItem {
              kind: "toolResult".to_string(),
              title: "write_file result".to_string(),
              content: format!("Wrote {} bytes to {}.", content.len(), relative_path),
              attributes: None,
            });
            items.push(TimelineItem {
              kind: "assistantMessage".to_string(),
              title: "Assistant".to_string(),
              content: format!(
                "Pith wrote {} in {} after your approval.",
                relative_path, workspace.display_name
              ),
              attributes: None,
            });
          }
          Err(error) => {
            items.push(TimelineItem {
              kind: "warning".to_string(),
              title: "write_file failed".to_string(),
              content: error.to_string(),
              attributes: None,
            });
          }
        }
      }
    } else if approval.action == "run_shell" {
      if !permission_is_granted(&permission_sources, "shell.exec") {
        items.extend(build_permission_denied_items(
          &permission_sources,
          "shell.exec",
          "complete the approved shell command",
          &workspace.display_name,
          HashMap::from([
            ("approvalId".to_string(), approval.id.clone()),
            (
              "command".to_string(),
              approval.command.clone().unwrap_or_default(),
            ),
          ]),
        ));
      } else {
        let command = approval.command.clone().unwrap_or_default();
        items.push(TimelineItem {
          kind: "toolStart".to_string(),
          title: "run_shell".to_string(),
          content: command.clone(),
          attributes: None,
        });

        match run_shell(Path::new(&workspace.root_path), &command, 4096) {
          Ok(result) => {
            memory_event = Some(MemoryEvent::ShellCommandRan {
              workspace_display_name: workspace.display_name.clone(),
              command: command.clone(),
            });
            let (summary, summary_attributes) = summarize_shell_result(
              &model_runtime,
              &memory_notes,
              &workspace.display_name,
              &result,
            );
            items.push(TimelineItem {
              kind: "toolResult".to_string(),
              title: "run_shell result".to_string(),
              content: format_shell_result(&result),
              attributes: Some(result.sandbox.attributes()),
            });
            items.push(TimelineItem {
              kind: "assistantMessage".to_string(),
              title: "Assistant".to_string(),
              content: summary,
              attributes: Some(summary_attributes),
            });
            let (hook_items, memory_captures) =
              build_shell_completed_hook_items(&plugins, &workspace, &command, &result);
            hook_memory_captures.extend(memory_captures);
            items.extend(hook_items);
          }
          Err(error) => {
            items.push(TimelineItem {
              kind: "warning".to_string(),
              title: "run_shell failed".to_string(),
              content: error.to_string(),
              attributes: None,
            });
          }
        }
      }
    }
  } else {
    memory_event = Some(MemoryEvent::ApprovalDenied {
      title: approval.title.clone(),
      action: approval.action.clone(),
    });
    let (summary, summary_attributes) = summarize_denied_approval(
      &model_runtime,
      &memory_notes,
      &workspace.display_name,
      &approval.action,
      &approval.relative_path,
      approval.command.as_deref(),
    );
    items.push(TimelineItem {
      kind: "approvalResolved".to_string(),
      title: "Approval Denied".to_string(),
      content: format!("Denied {} for {}.", approval.action, approval.relative_path),
      attributes: Some(HashMap::from([
        ("approvalId".to_string(), approval.id.clone()),
        ("decision".to_string(), "denied".to_string()),
      ])),
    });
    items.push(TimelineItem {
      kind: "assistantMessage".to_string(),
      title: "Assistant".to_string(),
      content: summary,
      attributes: Some(summary_attributes),
    });
  }

  ApprovalExecutionOutput {
    approval,
    decision,
    workspace,
    items,
    memory_event,
    hook_memory_captures,
  }
}

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

  let Some(thread) = context
    .threads
    .iter_mut()
    .find(|thread| thread.summary.id == approval.thread_id)
  else {
    return JsonRpcResponse::error(completed.request_id, -32004, "Thread not found");
  };
  thread.items.extend(items.clone());
  thread.summary.status = "Ready".to_string();

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
    if let Some(thread) = context
      .threads
      .iter_mut()
      .find(|thread| thread.summary.id == approval.thread_id)
    {
      thread.items.extend(hook_memory_items.clone());
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
