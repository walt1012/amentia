use std::collections::HashMap;
use std::path::Path;

use active_turns::{
  active_turn_id_for_thread, compute_streamed_char_count, start_streaming_assistant_turn,
  streaming_progress_label, update_streaming_item, ActiveTurn,
};
use anyhow::Result;
use intent_inference::{
  infer_requested_file_path, infer_search_query, infer_shell_command, infer_write_intent,
};
use local_responses::{
  build_plan_item, format_directory_result, format_file_result, format_search_result,
  format_shell_result, summarize_denied_approval, summarize_directory_result,
  summarize_file_result, summarize_search_result, summarize_shell_result,
};
use pith_memory::MemoryEvent;
use pith_protocol::{
  methods, ApprovalRequest, ApprovalRespondParams, ApprovalRespondResult, JsonRpcNotification,
  JsonRpcRequest, JsonRpcResponse, ThreadUpdatedNotificationParams, TimelineItem, TurnCancelParams,
  TurnCancelResult, TurnStartParams, TurnStartResult, WorkspaceSummary,
};
use pith_storage::StoredApprovalRecord;
use pith_tools::{
  generate_diff, list_directory, read_file, run_shell, search_files, shell_sandbox_summary,
  write_file,
};
use plugin_hooks::{build_shell_completed_hook_items, capture_plugin_hook_memory};
use plugin_permissions::{
  build_permission_denied_items, granted_permission_sources, permission_is_granted,
};
use request_params::parse_required_params;
pub(crate) use runtime_context::{
  ApprovalExecutionOutput, PendingApproval, PreparedApprovalSnapshot, PreparedTurnAction,
  PreparedTurnSnapshot, StoredThread, TurnStartExecutionOutput,
};
pub use runtime_context::{
  CompletedApprovalRespond, CompletedTurnStart, PreparedApprovalRespond, PreparedTurnStart,
  RuntimeContext,
};
use runtime_readiness::build_runtime_readiness;
use text_utils::{take_characters, truncate_text};

mod active_turns;
mod context_compaction;
mod context_state;
mod intent_inference;
mod local_responses;
mod memory_requests;
mod model_requests;
mod plugin_catalog_state;
mod plugin_commands;
mod plugin_hooks;
mod plugin_permissions;
mod plugin_requests;
mod protocol_adapters;
mod request_params;
mod runtime_context;
mod runtime_readiness;
mod server_requests;
mod text_utils;
mod thread_requests;
mod workspace_requests;
mod workspace_search;

pub use plugin_commands::{CompletedPluginCommandRun, PreparedPluginCommandRun};
pub use workspace_search::{CompletedWorkspaceSearch, PreparedWorkspaceSearch};

pub fn handle_request(context: &mut RuntimeContext, request: JsonRpcRequest) -> JsonRpcResponse {
  match request.method.as_str() {
    methods::APPROVAL_RESPOND => handle_approval_respond(context, request),
    methods::INITIALIZE => server_requests::handle_initialize(context, request),
    methods::HEALTH_PING => server_requests::handle_health_ping(request),
    methods::MEMORY_CREATE => memory_requests::handle_memory_create(context, request),
    methods::MEMORY_LIST => memory_requests::handle_memory_list(context, request),
    methods::MEMORY_STATUS => memory_requests::handle_memory_status(context, request),
    methods::MODEL_BOOTSTRAP => model_requests::handle_model_bootstrap(context, request),
    methods::MODEL_HEALTH => model_requests::handle_model_health(context, request),
    methods::PLUGIN_CAPABILITY_REGISTRY => {
      plugin_requests::handle_plugin_capability_registry(context, request)
    }
    methods::PLUGIN_COMMAND_REGISTRY => {
      plugin_requests::handle_plugin_command_registry(context, request)
    }
    methods::PLUGIN_COMMAND_RUN => plugin_commands::handle_plugin_command_run(context, request),
    methods::PLUGIN_CONNECTOR_REGISTRY => {
      plugin_requests::handle_plugin_connector_registry(context, request)
    }
    methods::PLUGIN_HOOK_REGISTRY => plugin_requests::handle_plugin_hook_registry(context, request),
    methods::PLUGIN_INSTALL => plugin_requests::handle_plugin_install(context, request),
    methods::PLUGIN_LIST => plugin_requests::handle_plugin_list(context, request),
    methods::PLUGIN_REMOVE => plugin_requests::handle_plugin_remove(context, request),
    methods::PLUGIN_SET_ENABLED => plugin_requests::handle_plugin_set_enabled(context, request),
    methods::RUNTIME_READINESS => {
      JsonRpcResponse::success(request.id, &build_runtime_readiness(context))
    }
    methods::WORKSPACE_CURRENT => workspace_requests::handle_workspace_current(context, request),
    methods::WORKSPACE_OPEN => workspace_requests::handle_workspace_open(context, request),
    methods::WORKSPACE_SEARCH => workspace_search::handle_workspace_search(context, request),
    methods::TURN_CANCEL => handle_turn_cancel(context, request),
    methods::THREAD_READ => thread_requests::handle_thread_read(context, request),
    methods::THREAD_START => thread_requests::handle_thread_start(context, request),
    methods::THREAD_LIST => thread_requests::handle_thread_list(context, request),
    methods::TURN_START => handle_turn_start(context, request),
    _ => JsonRpcResponse::error(request.id, -32601, "Method not found"),
  }
}

pub fn collect_notifications(context: &mut RuntimeContext) -> Result<Vec<JsonRpcNotification>> {
  let active_turn_ids = context.active_turns.keys().cloned().collect::<Vec<_>>();
  let mut notifications = vec![];
  let mut did_update = false;

  for turn_id in active_turn_ids {
    if let Some(params) = advance_active_turn(context, &turn_id)? {
      did_update = true;
      notifications.push(JsonRpcNotification {
        method: methods::THREAD_UPDATED_NOTIFICATION.to_string(),
        params: Some(serde_json::to_value(params)?),
      });
    }
  }

  if did_update {
    context.persist_runtime_state()?;
  }

  Ok(notifications)
}

fn handle_turn_start(context: &mut RuntimeContext, request: JsonRpcRequest) -> JsonRpcResponse {
  let prepared = match prepare_turn_start(context, request) {
    Ok(prepared) => prepared,
    Err(response) => return response,
  };
  let completed = execute_prepared_turn_start(prepared);
  complete_prepared_turn_start(context, completed)
}

pub fn prepare_turn_start(
  context: &mut RuntimeContext,
  request: JsonRpcRequest,
) -> std::result::Result<PreparedTurnStart, JsonRpcResponse> {
  let params = parse_required_params::<TurnStartParams>(&request, "turn/start")?;

  if let Err(message) = ensure_turn_model_ready(context) {
    return Err(JsonRpcResponse::error(request.id, -32060, message));
  }

  let current_workspace = context.workspace.clone();
  let model_runtime = context.model_runtime.clone();
  let memory_notes = context.memory_notes.clone();
  let permission_sources = granted_permission_sources(&context.plugins);
  let (thread_id, turn_id, thread_title, workspace) = {
    let Some(thread) = context
      .threads
      .iter_mut()
      .find(|thread| thread.summary.id == params.thread_id)
    else {
      return Err(JsonRpcResponse::error(
        request.id,
        -32004,
        "Thread not found",
      ));
    };

    thread.turn_count += 1;
    let turn_count = thread.turn_count;
    if thread.workspace.is_none() {
      thread.workspace = current_workspace.clone();
      thread.summary.workspace = thread.workspace.clone();
    }
    let workspace = thread.workspace.clone();
    thread.summary.status = match &workspace {
      Some(workspace) => format!("{turn_count} turn(s) in {}", workspace.display_name),
      None => format!("{turn_count} turn(s)"),
    };

    (
      thread.summary.id.clone(),
      format!("{}-turn-{turn_count}", thread.summary.id),
      thread.summary.title.clone(),
      workspace,
    )
  };
  let action = prepare_turn_action(
    context,
    &params.message,
    workspace.as_ref(),
    &permission_sources,
  );

  Ok(PreparedTurnStart {
    request_id: request.id,
    snapshot: PreparedTurnSnapshot {
      thread_id,
      turn_id,
      thread_title,
      display_message: params.message.clone(),
      message: params.message,
      workspace,
      model_runtime,
      memory_notes,
      permission_sources,
      action,
    },
  })
}

fn ensure_turn_model_ready(context: &RuntimeContext) -> std::result::Result<(), String> {
  if !context.enforce_model_readiness {
    return Ok(());
  }

  let health = context.model_runtime.health();
  if health.status == "ready" {
    return Ok(());
  }

  Err(format!(
    "Local model is not ready for turn/start. Download and activate a local model first. {}",
    health.detail
  ))
}

fn prepare_turn_action(
  context: &mut RuntimeContext,
  message: &str,
  workspace: Option<&WorkspaceSummary>,
  permission_sources: &HashMap<String, Vec<String>>,
) -> PreparedTurnAction {
  let Some(workspace) = workspace else {
    return PreparedTurnAction::NoWorkspace;
  };
  let workspace_root = Path::new(&workspace.root_path);

  if let Some(intent) = infer_write_intent(message) {
    let approval_id =
      permission_is_granted(permission_sources, "file.write").then(|| reserve_approval_id(context));
    return PreparedTurnAction::Write {
      intent,
      approval_id,
    };
  }

  if let Some(command) = infer_shell_command(message) {
    let approval_id =
      permission_is_granted(permission_sources, "shell.exec").then(|| reserve_approval_id(context));
    return PreparedTurnAction::Shell {
      command,
      approval_id,
    };
  }

  if let Some(relative_path) = infer_requested_file_path(message, workspace_root) {
    return PreparedTurnAction::ReadFile { relative_path };
  }

  if let Some(query) = infer_search_query(message) {
    return PreparedTurnAction::Search { query };
  }

  PreparedTurnAction::ListWorkspace
}

fn reserve_approval_id(context: &mut RuntimeContext) -> String {
  let approval_id = format!("approval-{}", context.next_approval_number);
  context.next_approval_number += 1;
  approval_id
}

pub fn execute_prepared_turn_start(prepared: PreparedTurnStart) -> CompletedTurnStart {
  CompletedTurnStart {
    request_id: prepared.request_id,
    output: execute_prepared_turn_snapshot(prepared.snapshot),
  }
}

fn execute_prepared_turn_snapshot(snapshot: PreparedTurnSnapshot) -> TurnStartExecutionOutput {
  let mut items = vec![TimelineItem {
    kind: "userMessage".to_string(),
    title: "User".to_string(),
    content: snapshot.display_message.clone(),
    attributes: None,
  }];
  let mut pending_active_turn = None;
  let mut pending_approval = None;

  match (&snapshot.workspace, &snapshot.action) {
    (
      Some(workspace),
      PreparedTurnAction::Write {
        intent,
        approval_id,
      },
    ) => {
      execute_write_turn(
        &snapshot,
        workspace,
        intent,
        approval_id,
        &mut items,
        &mut pending_approval,
      );
    }
    (
      Some(workspace),
      PreparedTurnAction::Shell {
        command,
        approval_id,
      },
    ) => {
      execute_shell_turn(
        &snapshot,
        workspace,
        command,
        approval_id,
        &mut items,
        &mut pending_approval,
      );
    }
    (Some(workspace), PreparedTurnAction::ReadFile { relative_path }) => {
      execute_read_turn(
        &snapshot,
        workspace,
        relative_path,
        &mut items,
        &mut pending_active_turn,
      );
    }
    (Some(workspace), PreparedTurnAction::Search { query }) => {
      execute_search_turn(
        &snapshot,
        workspace,
        query,
        &mut items,
        &mut pending_active_turn,
      );
    }
    (Some(workspace), PreparedTurnAction::ListWorkspace) => {
      execute_list_turn(&snapshot, workspace, &mut items, &mut pending_active_turn);
    }
    _ => execute_no_workspace_turn(&snapshot, &mut items),
  }

  TurnStartExecutionOutput {
    thread_id: snapshot.thread_id,
    turn_id: snapshot.turn_id,
    items,
    pending_approval,
    pending_active_turn,
  }
}

pub fn complete_prepared_turn_start(
  context: &mut RuntimeContext,
  completed: CompletedTurnStart,
) -> JsonRpcResponse {
  let output = completed.output;
  let active_turn_id = output
    .pending_active_turn
    .as_ref()
    .map(|turn| turn.id.clone());

  if let Some(approval) = output.pending_approval.clone() {
    context
      .pending_approvals
      .insert(approval.id.clone(), approval);
  }

  let Some(thread) = context
    .threads
    .iter_mut()
    .find(|thread| thread.summary.id == output.thread_id)
  else {
    return JsonRpcResponse::error(completed.request_id, -32004, "Thread not found");
  };

  if active_turn_id.is_some() {
    thread.summary.status = "Streaming assistant response".to_string();
  } else if !thread.summary.status.contains("approval") {
    thread.summary.status = "Ready".to_string();
  }
  thread.items.extend(output.items.clone());

  if let Some(active_turn) = output.pending_active_turn {
    context
      .active_turns
      .insert(active_turn.id.clone(), active_turn);
  }

  if let Err(error) = context.persist_runtime_state() {
    return JsonRpcResponse::error(completed.request_id, -32010, error.to_string());
  }

  if active_turn_id.is_none() {
    if let Err(error) = refresh_thread_summary_note(context, &output.thread_id) {
      return JsonRpcResponse::error(completed.request_id, -32012, error.to_string());
    }
  }

  JsonRpcResponse::success(
    completed.request_id,
    &TurnStartResult {
      turn_id: output.turn_id,
      thread_id: output.thread_id.clone(),
      items: output.items,
      pending_approvals: approvals_for_thread(context, &output.thread_id),
      active_turn_id,
    },
  )
}

fn execute_write_turn(
  snapshot: &PreparedTurnSnapshot,
  workspace: &WorkspaceSummary,
  intent: &intent_inference::WriteIntent,
  approval_id: &Option<String>,
  items: &mut Vec<TimelineItem>,
  pending_approval: &mut Option<PendingApproval>,
) {
  items.push(build_plan_item(
    &snapshot.model_runtime,
    &snapshot.memory_notes,
    &snapshot.message,
    Some(workspace),
    if approval_id.is_some() {
      format!(
        "Request approval before writing {} in {}.",
        intent.relative_path, workspace.display_name
      )
    } else {
      format!(
        "Check plugin permissions before writing {} in {}.",
        intent.relative_path, workspace.display_name
      )
    },
  ));
  let Some(approval_id) = approval_id else {
    items.extend(build_permission_denied_items(
      &snapshot.permission_sources,
      "file.write",
      "prepare a file write",
      &workspace.display_name,
      HashMap::from([("relativePath".to_string(), intent.relative_path.clone())]),
    ));
    return;
  };

  let approval = PendingApproval {
    id: approval_id.clone(),
    thread_id: snapshot.thread_id.clone(),
    action: "write_file".to_string(),
    title: format!("Write {}", intent.relative_path),
    relative_path: intent.relative_path.clone(),
    content: Some(intent.content.clone()),
    command: None,
  };
  *pending_approval = Some(approval.clone());

  items.push(TimelineItem {
    kind: "toolStart".to_string(),
    title: "generate_diff".to_string(),
    content: intent.relative_path.clone(),
    attributes: None,
  });
  match generate_diff(
    Path::new(&workspace.root_path),
    &intent.relative_path,
    &intent.content,
  ) {
    Ok(diff) => {
      items.push(TimelineItem {
        kind: "diffArtifact".to_string(),
        title: "Diff Preview".to_string(),
        content: diff,
        attributes: Some(HashMap::from([
          ("action".to_string(), "write_file".to_string()),
          ("relativePath".to_string(), intent.relative_path.clone()),
        ])),
      });
    }
    Err(error) => {
      items.push(TimelineItem {
        kind: "warning".to_string(),
        title: "generate_diff failed".to_string(),
        content: error.to_string(),
        attributes: None,
      });
    }
  }
  items.push(TimelineItem {
    kind: "approvalRequested".to_string(),
    title: "Approval Requested".to_string(),
    content: format!(
      "Pith wants to write {} in {}.",
      intent.relative_path, workspace.display_name
    ),
    attributes: Some(HashMap::from([
      ("approvalId".to_string(), approval.id.clone()),
      ("action".to_string(), approval.action.clone()),
      ("relativePath".to_string(), approval.relative_path.clone()),
    ])),
  });
  items.push(TimelineItem {
    kind: "assistantMessage".to_string(),
    title: "Assistant".to_string(),
    content: format!(
      "Pith prepared a write for {} and is waiting for your approval.",
      intent.relative_path
    ),
    attributes: None,
  });
}

fn execute_shell_turn(
  snapshot: &PreparedTurnSnapshot,
  workspace: &WorkspaceSummary,
  command: &str,
  approval_id: &Option<String>,
  items: &mut Vec<TimelineItem>,
  pending_approval: &mut Option<PendingApproval>,
) {
  items.push(build_plan_item(
    &snapshot.model_runtime,
    &snapshot.memory_notes,
    &snapshot.message,
    Some(workspace),
    if approval_id.is_some() {
      format!(
        "Request approval before running a shell command in {}.",
        workspace.display_name
      )
    } else {
      format!(
        "Check plugin permissions before running a shell command in {}.",
        workspace.display_name
      )
    },
  ));
  let Some(approval_id) = approval_id else {
    items.extend(build_permission_denied_items(
      &snapshot.permission_sources,
      "shell.exec",
      "run a shell command",
      &workspace.display_name,
      HashMap::from([("command".to_string(), command.to_string())]),
    ));
    return;
  };

  let sandbox = shell_sandbox_summary(Path::new(&workspace.root_path));
  let approval = PendingApproval {
    id: approval_id.clone(),
    thread_id: snapshot.thread_id.clone(),
    action: "run_shell".to_string(),
    title: "Run Shell Command".to_string(),
    relative_path: ".".to_string(),
    content: None,
    command: Some(command.to_string()),
  };
  *pending_approval = Some(approval.clone());

  items.push(TimelineItem {
    kind: "approvalRequested".to_string(),
    title: "Approval Requested".to_string(),
    content: format!(
      "Pith wants to run this shell command in {}:\n{}\n\n{}",
      workspace.display_name,
      command,
      sandbox.display_line()
    ),
    attributes: Some({
      let mut attributes = sandbox.attributes();
      attributes.extend(HashMap::from([
        ("approvalId".to_string(), approval.id.clone()),
        ("action".to_string(), approval.action.clone()),
        ("command".to_string(), command.to_string()),
      ]));
      attributes
    }),
  });
  items.push(TimelineItem {
    kind: "assistantMessage".to_string(),
    title: "Assistant".to_string(),
    content: "Pith is waiting for your approval before running the shell command.".to_string(),
    attributes: None,
  });
}

fn execute_read_turn(
  snapshot: &PreparedTurnSnapshot,
  workspace: &WorkspaceSummary,
  relative_path: &str,
  items: &mut Vec<TimelineItem>,
  pending_active_turn: &mut Option<ActiveTurn>,
) {
  items.push(build_plan_item(
    &snapshot.model_runtime,
    &snapshot.memory_notes,
    &snapshot.message,
    Some(workspace),
    if permission_is_granted(&snapshot.permission_sources, "file.read") {
      format!(
        "Inspect {} in {} with the built-in read_file tool.",
        relative_path, workspace.display_name
      )
    } else {
      format!(
        "Check plugin permissions before inspecting {} in {}.",
        relative_path, workspace.display_name
      )
    },
  ));
  if !permission_is_granted(&snapshot.permission_sources, "file.read") {
    items.extend(build_permission_denied_items(
      &snapshot.permission_sources,
      "file.read",
      "inspect a file",
      &workspace.display_name,
      HashMap::from([("relativePath".to_string(), relative_path.to_string())]),
    ));
    return;
  }

  items.push(TimelineItem {
    kind: "toolStart".to_string(),
    title: "read_file".to_string(),
    content: relative_path.to_string(),
    attributes: None,
  });

  match read_file(Path::new(&workspace.root_path), relative_path, 4096) {
    Ok(result) => {
      items.push(TimelineItem {
        kind: "toolResult".to_string(),
        title: "read_file result".to_string(),
        content: format_file_result(&result),
        attributes: None,
      });
      let (summary, summary_attributes) = summarize_file_result(
        &snapshot.model_runtime,
        &snapshot.memory_notes,
        &snapshot.thread_title,
        &workspace.display_name,
        &result,
      );
      *pending_active_turn = start_streaming_assistant_turn(
        &snapshot.thread_id,
        &snapshot.turn_id,
        items,
        summary,
        summary_attributes,
      );
    }
    Err(error) => {
      items.push(TimelineItem {
        kind: "warning".to_string(),
        title: "read_file failed".to_string(),
        content: error.to_string(),
        attributes: None,
      });
      items.push(TimelineItem {
        kind: "assistantMessage".to_string(),
        title: "Assistant".to_string(),
        content: format!(
          "Pith could not inspect that file in {}. Try another path inside the workspace.",
          workspace.display_name
        ),
        attributes: None,
      });
    }
  }
}

fn execute_search_turn(
  snapshot: &PreparedTurnSnapshot,
  workspace: &WorkspaceSummary,
  query: &str,
  items: &mut Vec<TimelineItem>,
  pending_active_turn: &mut Option<ActiveTurn>,
) {
  items.push(build_plan_item(
    &snapshot.model_runtime,
    &snapshot.memory_notes,
    &snapshot.message,
    Some(workspace),
    if permission_is_granted(&snapshot.permission_sources, "file.read") {
      format!(
        "Search {} for matches to \"{}\" with the built-in search_files tool.",
        workspace.display_name, query
      )
    } else {
      format!(
        "Check plugin permissions before searching {} for \"{}\".",
        workspace.display_name, query
      )
    },
  ));
  if !permission_is_granted(&snapshot.permission_sources, "file.read") {
    items.extend(build_permission_denied_items(
      &snapshot.permission_sources,
      "file.read",
      "search files",
      &workspace.display_name,
      HashMap::from([("query".to_string(), query.to_string())]),
    ));
    return;
  }

  items.push(TimelineItem {
    kind: "toolStart".to_string(),
    title: "search_files".to_string(),
    content: query.to_string(),
    attributes: None,
  });

  match search_files(Path::new(&workspace.root_path), query, 12) {
    Ok(matches) => {
      items.push(TimelineItem {
        kind: "toolResult".to_string(),
        title: "search_files result".to_string(),
        content: format_search_result(query, &matches),
        attributes: None,
      });
      let (summary, summary_attributes) = summarize_search_result(
        &snapshot.model_runtime,
        &snapshot.memory_notes,
        &snapshot.thread_title,
        &workspace.display_name,
        query,
        &matches,
      );
      *pending_active_turn = start_streaming_assistant_turn(
        &snapshot.thread_id,
        &snapshot.turn_id,
        items,
        summary,
        summary_attributes,
      );
    }
    Err(error) => {
      items.push(TimelineItem {
        kind: "warning".to_string(),
        title: "search_files failed".to_string(),
        content: error.to_string(),
        attributes: None,
      });
      items.push(TimelineItem {
        kind: "assistantMessage".to_string(),
        title: "Assistant".to_string(),
        content: format!(
          "Pith could not search {} yet. Try a shorter query or re-open the workspace.",
          workspace.display_name
        ),
        attributes: None,
      });
    }
  }
}

fn execute_list_turn(
  snapshot: &PreparedTurnSnapshot,
  workspace: &WorkspaceSummary,
  items: &mut Vec<TimelineItem>,
  pending_active_turn: &mut Option<ActiveTurn>,
) {
  items.push(build_plan_item(
    &snapshot.model_runtime,
    &snapshot.memory_notes,
    &snapshot.message,
    Some(workspace),
    if permission_is_granted(&snapshot.permission_sources, "file.read") {
      format!(
        "Inspect the root of {} with the built-in list_directory tool.",
        workspace.display_name
      )
    } else {
      format!(
        "Check plugin permissions before inspecting the root of {}.",
        workspace.display_name
      )
    },
  ));
  if !permission_is_granted(&snapshot.permission_sources, "file.read") {
    items.extend(build_permission_denied_items(
      &snapshot.permission_sources,
      "file.read",
      "inspect the workspace",
      &workspace.display_name,
      HashMap::new(),
    ));
    return;
  }

  items.push(TimelineItem {
    kind: "toolStart".to_string(),
    title: "list_directory".to_string(),
    content: ".".to_string(),
    attributes: None,
  });

  match list_directory(Path::new(&workspace.root_path), None, 24) {
    Ok(entries) => {
      items.push(TimelineItem {
        kind: "toolResult".to_string(),
        title: "list_directory result".to_string(),
        content: format_directory_result(&entries),
        attributes: None,
      });
      let (summary, summary_attributes) = summarize_directory_result(
        &snapshot.model_runtime,
        &snapshot.memory_notes,
        &snapshot.thread_title,
        &workspace.display_name,
        &entries,
      );
      *pending_active_turn = start_streaming_assistant_turn(
        &snapshot.thread_id,
        &snapshot.turn_id,
        items,
        summary,
        summary_attributes,
      );
    }
    Err(error) => {
      items.push(TimelineItem {
        kind: "warning".to_string(),
        title: "list_directory failed".to_string(),
        content: error.to_string(),
        attributes: None,
      });
      items.push(TimelineItem {
        kind: "assistantMessage".to_string(),
        title: "Assistant".to_string(),
        content: format!(
          "Pith could not inspect the root of {} yet. Re-open the workspace and try again.",
          workspace.display_name
        ),
        attributes: None,
      });
    }
  }
}

fn execute_no_workspace_turn(snapshot: &PreparedTurnSnapshot, items: &mut Vec<TimelineItem>) {
  items.push(build_plan_item(
    &snapshot.model_runtime,
    &snapshot.memory_notes,
    &snapshot.message,
    None,
    "Wait for a workspace before running filesystem tools.".to_string(),
  ));
  items.push(TimelineItem {
    kind: "warning".to_string(),
    title: "Workspace Required".to_string(),
    content: "Open a workspace before asking Pith to inspect files.".to_string(),
    attributes: None,
  });
  items.push(TimelineItem {
    kind: "assistantMessage".to_string(),
    title: "Assistant".to_string(),
    content: format!(
      "Pith received your message in {}, but project tools need an opened workspace first.",
      snapshot.thread_title
    ),
    attributes: None,
  });
}

fn handle_approval_respond(
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

  let Some(approval) = context.pending_approvals.get(&params.approval_id).cloned() else {
    return Err(JsonRpcResponse::error(
      request.id,
      -32030,
      "Approval request not found",
    ));
  };
  let current_workspace = context.workspace.clone();
  let model_runtime = context.model_runtime.clone();
  let memory_notes = context.memory_notes.clone();
  let permission_sources = granted_permission_sources(&context.plugins);
  let plugins = context.plugins.clone();

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
  context.pending_approvals.remove(&params.approval_id);

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

pub fn prepare_workspace_search(
  context: &mut RuntimeContext,
  request: JsonRpcRequest,
) -> std::result::Result<PreparedWorkspaceSearch, JsonRpcResponse> {
  workspace_search::prepare_workspace_search(context, request)
}

pub fn execute_prepared_workspace_search(
  prepared: PreparedWorkspaceSearch,
) -> CompletedWorkspaceSearch {
  workspace_search::execute_prepared_workspace_search(prepared)
}

pub fn complete_prepared_workspace_search(completed: CompletedWorkspaceSearch) -> JsonRpcResponse {
  workspace_search::complete_prepared_workspace_search(completed)
}

pub fn prepare_plugin_command_run(
  context: &mut RuntimeContext,
  request: JsonRpcRequest,
) -> std::result::Result<PreparedPluginCommandRun, JsonRpcResponse> {
  plugin_commands::prepare_plugin_command_run(context, request)
}

pub fn execute_prepared_plugin_command_run(
  prepared: PreparedPluginCommandRun,
) -> CompletedPluginCommandRun {
  plugin_commands::execute_prepared_plugin_command_run(prepared)
}

pub fn complete_prepared_plugin_command_run(
  context: &mut RuntimeContext,
  completed: CompletedPluginCommandRun,
) -> JsonRpcResponse {
  plugin_commands::complete_prepared_plugin_command_run(context, completed)
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

fn handle_turn_cancel(context: &mut RuntimeContext, request: JsonRpcRequest) -> JsonRpcResponse {
  let params = match parse_required_params::<TurnCancelParams>(&request, "turn/cancel") {
    Ok(params) => params,
    Err(response) => return response,
  };

  let Some(active_turn_snapshot) = context.active_turns.get(&params.turn_id).cloned() else {
    return JsonRpcResponse::error(request.id, -32040, "Turn is not active");
  };

  let Some(thread) = context
    .threads
    .iter_mut()
    .find(|thread| thread.summary.id == active_turn_snapshot.thread_id)
  else {
    return JsonRpcResponse::error(request.id, -32004, "Thread not found");
  };
  let cancelled_thread_id = thread.summary.id.clone();

  context.active_turns.remove(&params.turn_id);
  let partial_content = take_characters(
    &active_turn_snapshot.full_content,
    compute_streamed_char_count(&active_turn_snapshot)
      .min(active_turn_snapshot.full_content.chars().count()),
  );
  update_streaming_item(
    &mut thread.items,
    &params.turn_id,
    &partial_content,
    "cancelled",
    partial_content.chars().count(),
    active_turn_snapshot.total_chars,
  );
  thread.summary.status = "Turn cancelled".to_string();

  let items = vec![
    TimelineItem {
      kind: "warning".to_string(),
      title: "Turn Cancelled".to_string(),
      content: format!(
        "Cancelled {} before the assistant response completed.",
        params.turn_id
      ),
      attributes: Some(HashMap::from([(
        "turnId".to_string(),
        params.turn_id.clone(),
      )])),
    },
    TimelineItem {
      kind: "assistantMessage".to_string(),
      title: "Assistant".to_string(),
      content: "Pith stopped the active response at your request.".to_string(),
      attributes: Some(HashMap::from([
        ("turnId".to_string(), params.turn_id.clone()),
        ("streamingStatus".to_string(), "cancelled".to_string()),
      ])),
    },
  ];
  thread.items.extend(items.clone());

  if let Err(error) = context.persist_threads() {
    return JsonRpcResponse::error(request.id, -32010, error.to_string());
  }

  if let Err(error) = refresh_thread_summary_note(context, &cancelled_thread_id) {
    return JsonRpcResponse::error(request.id, -32012, error.to_string());
  }

  JsonRpcResponse::success(
    request.id,
    &TurnCancelResult {
      turn_id: params.turn_id,
      thread_id: active_turn_snapshot.thread_id,
      items,
      active_turn_id: active_turn_id_for_thread(&context.active_turns, &cancelled_thread_id),
    },
  )
}

fn refresh_thread_summary_note(context: &mut RuntimeContext, thread_id: &str) -> Result<()> {
  let Some(thread) = context
    .threads
    .iter()
    .find(|thread| thread.summary.id == thread_id)
    .cloned()
  else {
    return Ok(());
  };

  let pending_approvals = approvals_for_thread(context, thread_id);
  let workspace_snapshot = thread.workspace.clone();
  let scope = thread
    .workspace
    .as_ref()
    .map(|workspace| workspace.display_name.clone())
    .unwrap_or_else(|| "global".to_string());

  context.upsert_memory_note(
    format!("memory-thread-summary-{thread_id}"),
    format!("Thread summary: {}", thread.summary.title),
    build_thread_summary_body(&thread, workspace_snapshot.as_ref(), &pending_approvals),
    scope,
    "thread".to_string(),
    vec![
      "thread".to_string(),
      "summary".to_string(),
      thread_id.to_string(),
    ],
  )?;

  Ok(())
}

fn build_thread_summary_body(
  thread: &StoredThread,
  workspace: Option<&WorkspaceSummary>,
  pending_approvals: &[ApprovalRequest],
) -> String {
  let workspace_line = workspace
    .map(|workspace| format!("Workspace: {}.", workspace.display_name))
    .unwrap_or_else(|| "Workspace: unavailable.".to_string());
  let latest_user = thread
    .items
    .iter()
    .rev()
    .find(|item| item.kind == "userMessage")
    .map(|item| truncate_text(&item.content, 180))
    .unwrap_or_else(|| "No user request captured yet.".to_string());
  let latest_assistant = thread
    .items
    .iter()
    .rev()
    .find(|item| item.kind == "assistantMessage")
    .map(|item| truncate_text(&item.content, 180))
    .unwrap_or_else(|| "No assistant update captured yet.".to_string());
  let recent_activity = thread
    .items
    .iter()
    .rev()
    .filter(|item| item.kind != "system")
    .take(4)
    .map(|item| item.title.clone())
    .collect::<Vec<_>>();
  let activity_line = if recent_activity.is_empty() {
    "Recent activity: none yet.".to_string()
  } else {
    format!("Recent activity: {}.", recent_activity.join(", "))
  };

  format!(
    "{workspace_line}\nStatus: {}.\nLast user request: {}.\nLatest assistant update: {}.\nPending approvals: {}.\n{}",
    thread.summary.status,
    latest_user,
    latest_assistant,
    pending_approvals.len(),
    activity_line
  )
}

pub(crate) fn refresh_active_turn_for_thread(
  context: &mut RuntimeContext,
  thread_id: &str,
) -> Result<bool> {
  let active_turn_ids = context
    .active_turns
    .values()
    .filter(|turn| turn.thread_id == thread_id)
    .map(|turn| turn.id.clone())
    .collect::<Vec<_>>();
  let mut did_update = false;

  for turn_id in active_turn_ids {
    if advance_active_turn(context, &turn_id)?.is_some() {
      did_update = true;
    }
  }

  Ok(did_update)
}

fn advance_active_turn(
  context: &mut RuntimeContext,
  turn_id: &str,
) -> Result<Option<ThreadUpdatedNotificationParams>> {
  let Some(snapshot) = context.active_turns.get(turn_id).cloned() else {
    return Ok(None);
  };
  let target_chars = compute_streamed_char_count(&snapshot).min(snapshot.total_chars);

  if target_chars <= snapshot.emitted_chars {
    return Ok(None);
  }

  let thread_id = snapshot.thread_id.clone();
  let streamed_content = take_characters(&snapshot.full_content, target_chars);
  let is_complete = target_chars >= snapshot.total_chars;
  let streaming_status = if is_complete {
    "completed"
  } else {
    "in_progress"
  };

  let thread_snapshot = {
    let Some(thread) = context
      .threads
      .iter_mut()
      .find(|thread| thread.summary.id == snapshot.thread_id)
    else {
      return Ok(None);
    };

    update_streaming_item(
      &mut thread.items,
      turn_id,
      &streamed_content,
      streaming_status,
      target_chars,
      snapshot.total_chars,
    );

    if is_complete {
      thread.summary.status = "Ready".to_string();
    } else {
      thread.summary.status = format!(
        "Streaming assistant response ({})",
        streaming_progress_label(target_chars, snapshot.total_chars)
      );
    }

    (thread.summary.clone(), thread.items.clone())
  };

  if is_complete {
    context.active_turns.remove(turn_id);
    refresh_thread_summary_note(context, &thread_id)?;
  } else if let Some(active_turn) = context.active_turns.get_mut(turn_id) {
    active_turn.emitted_chars = target_chars;
  }

  Ok(Some(ThreadUpdatedNotificationParams {
    thread: thread_snapshot.0,
    items: thread_snapshot.1,
    pending_approvals: approvals_for_thread(context, &thread_id),
    active_turn_id: active_turn_id_for_thread(&context.active_turns, &thread_id),
  }))
}

fn stored_approval_record(approval: PendingApproval) -> StoredApprovalRecord {
  StoredApprovalRecord {
    id: approval.id,
    thread_id: approval.thread_id,
    action: approval.action,
    title: approval.title,
    relative_path: approval.relative_path,
    content: approval.content,
    command: approval.command,
  }
}

pub(crate) fn approvals_for_thread(
  context: &RuntimeContext,
  thread_id: &str,
) -> Vec<ApprovalRequest> {
  let mut approvals = context
    .pending_approvals
    .values()
    .filter(|approval| approval.thread_id == thread_id)
    .map(|approval| ApprovalRequest {
      id: approval.id.clone(),
      thread_id: approval.thread_id.clone(),
      action: approval.action.clone(),
      title: approval.title.clone(),
      relative_path: approval.relative_path.clone(),
    })
    .collect::<Vec<_>>();
  approvals.sort_by(|left, right| left.id.cmp(&right.id));
  approvals
}

#[cfg(test)]
mod tests;
