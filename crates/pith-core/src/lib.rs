use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

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
use pith_memory::{MemoryEvent, MemoryManager, MemoryNote};
use pith_model_runtime::LocalModelRuntime;
use pith_plugin_host::{
  inspect_plugin_bundle, install_plugin_bundle, remove_local_plugin_bundle, PluginCatalogEntry,
};
use pith_protocol::{
  methods, ApprovalRequest, ApprovalRespondParams, ApprovalRespondResult, HealthPingResult,
  InitializeParams, InitializeResult, JsonRpcNotification, JsonRpcRequest, JsonRpcResponse,
  MemoryCreateParams, MemoryCreateResult, MemoryListResult, PluginInstallParams,
  PluginInstallResult, PluginListResult, PluginRemoveParams, PluginRemoveResult,
  PluginSetEnabledParams, PluginSetEnabledResult, ServerCapabilities, ServerInfo,
  ThreadListResult, ThreadReadParams, ThreadReadResult, ThreadStartParams,
  ThreadStartResult, ThreadSummary, ThreadUpdatedNotificationParams, TimelineItem,
  TurnCancelParams, TurnCancelResult, TurnStartParams, TurnStartResult, WorkspaceCurrentResult,
  WorkspaceOpenParams, WorkspaceOpenResult, WorkspaceSearchMatch, WorkspaceSearchParams,
  WorkspaceSearchResult, WorkspaceSummary,
};
use pith_storage::{FileThreadStore, StoredApprovalRecord};
use pith_tools::{
  generate_diff, list_directory, read_file, run_shell, search_files, shell_sandbox_summary,
  write_file,
};
use plugin_hooks::{
  build_plugin_hook_memory_body, build_shell_completed_hook_items, plugin_hook_memory_tags,
  PluginHookMemoryCapture,
};
use plugin_permissions::{
  build_permission_denied_items, granted_permission_sources, permission_is_granted,
};
use protocol_adapters::{
  build_protocol_capability_registry, build_protocol_command_registry,
  build_protocol_connector_registry, build_protocol_hook_registry, to_protocol_memory_note,
  to_protocol_memory_status, to_protocol_model_bootstrap, to_protocol_model_health,
  to_protocol_plugin,
};
use request_params::parse_required_params;
use runtime_readiness::build_runtime_readiness;
use text_utils::{take_characters, truncate_text};

mod active_turns;
mod context_compaction;
mod context_state;
mod intent_inference;
mod local_responses;
mod plugin_catalog_state;
mod plugin_commands;
mod plugin_hooks;
mod plugin_permissions;
mod protocol_adapters;
mod request_params;
mod runtime_readiness;
mod text_utils;

#[derive(Debug, Clone)]
struct StoredThread {
  summary: ThreadSummary,
  turn_count: usize,
  items: Vec<TimelineItem>,
  workspace: Option<WorkspaceSummary>,
}

#[derive(Debug, Clone)]
struct PendingApproval {
  id: String,
  thread_id: String,
  action: String,
  title: String,
  relative_path: String,
  content: Option<String>,
  command: Option<String>,
}

#[derive(Debug, Clone)]
pub struct RuntimeContext {
  server_name: String,
  server_version: String,
  model_runtime: LocalModelRuntime,
  memory_manager: MemoryManager,
  store: Option<FileThreadStore>,
  memory_notes: Vec<MemoryNote>,
  threads: Vec<StoredThread>,
  workspace: Option<WorkspaceSummary>,
  plugin_roots: Vec<PathBuf>,
  plugin_install_root: PathBuf,
  plugins: Vec<PluginCatalogEntry>,
  pending_approvals: HashMap<String, PendingApproval>,
  active_turns: HashMap<String, ActiveTurn>,
  next_thread_number: usize,
  next_approval_number: usize,
}

pub fn handle_request(context: &mut RuntimeContext, request: JsonRpcRequest) -> JsonRpcResponse {
  match request.method.as_str() {
    methods::APPROVAL_RESPOND => handle_approval_respond(context, request),
    methods::INITIALIZE => handle_initialize(context, request),
    methods::HEALTH_PING => JsonRpcResponse::success(
      request.id,
      &HealthPingResult {
        status: "ok".to_string(),
      },
    ),
    methods::MEMORY_CREATE => handle_memory_create(context, request),
    methods::MEMORY_LIST => JsonRpcResponse::success(
      request.id,
      &MemoryListResult {
        notes: context
          .memory_notes
          .iter()
          .take(16)
          .cloned()
          .map(to_protocol_memory_note)
          .collect(),
      },
    ),
    methods::MEMORY_STATUS => JsonRpcResponse::success(
      request.id,
      &to_protocol_memory_status(context.memory_manager.status(&context.memory_notes)),
    ),
    methods::MODEL_BOOTSTRAP => handle_model_bootstrap(context, request),
    methods::MODEL_HEALTH => JsonRpcResponse::success(
      request.id,
      &to_protocol_model_health(context.model_runtime.health()),
    ),
    methods::PLUGIN_CAPABILITY_REGISTRY => JsonRpcResponse::success(
      request.id,
      &build_protocol_capability_registry(&context.plugins),
    ),
    methods::PLUGIN_COMMAND_REGISTRY => JsonRpcResponse::success(
      request.id,
      &build_protocol_command_registry(&context.plugins),
    ),
    methods::PLUGIN_COMMAND_RUN => plugin_commands::handle_plugin_command_run(context, request),
    methods::PLUGIN_CONNECTOR_REGISTRY => JsonRpcResponse::success(
      request.id,
      &build_protocol_connector_registry(&context.plugins),
    ),
    methods::PLUGIN_HOOK_REGISTRY => {
      JsonRpcResponse::success(request.id, &build_protocol_hook_registry(&context.plugins))
    }
    methods::PLUGIN_INSTALL => handle_plugin_install(context, request),
    methods::PLUGIN_LIST => JsonRpcResponse::success(
      request.id,
      &PluginListResult {
        plugins: context
          .plugins
          .iter()
          .cloned()
          .map(to_protocol_plugin)
          .collect(),
      },
    ),
    methods::PLUGIN_REMOVE => handle_plugin_remove(context, request),
    methods::PLUGIN_SET_ENABLED => handle_plugin_set_enabled(context, request),
    methods::RUNTIME_READINESS => {
      JsonRpcResponse::success(request.id, &build_runtime_readiness(context))
    }
    methods::WORKSPACE_CURRENT => JsonRpcResponse::success(
      request.id,
      &WorkspaceCurrentResult {
        workspace: context.workspace.clone(),
      },
    ),
    methods::WORKSPACE_OPEN => handle_workspace_open(context, request),
    methods::WORKSPACE_SEARCH => handle_workspace_search(context, request),
    methods::TURN_CANCEL => handle_turn_cancel(context, request),
    methods::THREAD_READ => handle_thread_read(context, request),
    methods::THREAD_START => handle_thread_start(context, request),
    methods::THREAD_LIST => JsonRpcResponse::success(
      request.id,
      &ThreadListResult {
        threads: context
          .threads
          .iter()
          .map(|thread| thread.summary.clone())
          .collect(),
      },
    ),
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

fn handle_initialize(context: &RuntimeContext, request: JsonRpcRequest) -> JsonRpcResponse {
  let params = match parse_required_params::<InitializeParams>(&request, "initialize") {
    Ok(params) => params,
    Err(response) => return response,
  };

  let _client = params.client_info;

  JsonRpcResponse::success(
    request.id,
    &InitializeResult {
      server_info: ServerInfo {
        name: context.server_name.clone(),
        version: context.server_version.clone(),
      },
      protocol_version: "0.1.0".to_string(),
      capabilities: ServerCapabilities {
        supports_memory: true,
        supports_threads: true,
        supports_tools: true,
        supports_plugins: !context.plugins.is_empty(),
        supports_runtime_readiness: true,
      },
    },
  )
}

fn handle_memory_create(context: &mut RuntimeContext, request: JsonRpcRequest) -> JsonRpcResponse {
  let params = match parse_required_params::<MemoryCreateParams>(&request, "memory/create") {
    Ok(params) => params,
    Err(response) => return response,
  };

  let Some(workspace) = context.workspace.clone() else {
    return JsonRpcResponse::error(
      request.id,
      -32040,
      "Open a workspace before creating memory notes",
    );
  };

  let title = params.title.trim();
  let body = params.body.trim();
  if title.is_empty() || body.is_empty() {
    return JsonRpcResponse::error(
      request.id,
      -32602,
      "memory/create title and body must be non-empty",
    );
  }

  match context.create_memory_note(
    title.to_string(),
    body.to_string(),
    workspace.display_name,
    "user".to_string(),
    vec![
      "workspace".to_string(),
      "user".to_string(),
      "manual".to_string(),
    ],
  ) {
    Ok(note) => JsonRpcResponse::success(
      request.id,
      &MemoryCreateResult {
        note: to_protocol_memory_note(note),
      },
    ),
    Err(error) => JsonRpcResponse::error(request.id, -32041, error.to_string()),
  }
}

fn handle_model_bootstrap(
  context: &mut RuntimeContext,
  request: JsonRpcRequest,
) -> JsonRpcResponse {
  match context.model_runtime.bootstrap_pack_metadata() {
    Ok(result) => {
      context.model_runtime = LocalModelRuntime::new_default();
      JsonRpcResponse::success(request.id, &to_protocol_model_bootstrap(result))
    }
    Err(error) => JsonRpcResponse::error(request.id, -32042, error.to_string()),
  }
}

fn handle_plugin_set_enabled(
  context: &mut RuntimeContext,
  request: JsonRpcRequest,
) -> JsonRpcResponse {
  let params = match parse_required_params::<PluginSetEnabledParams>(&request, "plugin/setEnabled")
  {
    Ok(params) => params,
    Err(response) => return response,
  };

  let Some(plugin_index) = context
    .plugins
    .iter()
    .position(|plugin| plugin.id == params.plugin_id)
  else {
    return JsonRpcResponse::error(request.id, -32050, "Plugin not found");
  };
  if context.plugins[plugin_index].status != "ready" {
    return JsonRpcResponse::error(
      request.id,
      -32051,
      context.plugins[plugin_index]
        .validation_error
        .clone()
        .unwrap_or_else(|| "Plugin manifest is invalid".to_string()),
    );
  }

  context.plugins[plugin_index].enabled = params.enabled;
  let plugin_id = context.plugins[plugin_index].id.clone();
  let plugin_enabled = context.plugins[plugin_index].enabled;
  let updated_plugin = context.plugins[plugin_index].clone();

  if let Err(error) = context.persist_plugin_enabled(&plugin_id, plugin_enabled) {
    return JsonRpcResponse::error(request.id, -32010, error.to_string());
  }

  JsonRpcResponse::success(
    request.id,
    &PluginSetEnabledResult {
      plugin: to_protocol_plugin(updated_plugin),
    },
  )
}

fn handle_plugin_install(context: &mut RuntimeContext, request: JsonRpcRequest) -> JsonRpcResponse {
  let params = match parse_required_params::<PluginInstallParams>(&request, "plugin/install") {
    Ok(params) => params,
    Err(response) => return response,
  };

  let source_path = PathBuf::from(&params.source_path);
  let candidate_plugin = match inspect_plugin_bundle(&source_path) {
    Ok(plugin) => plugin,
    Err(error) => return JsonRpcResponse::error(request.id, -32053, error.to_string()),
  };
  if context
    .plugins
    .iter()
    .any(|plugin| plugin.id == candidate_plugin.id)
  {
    return JsonRpcResponse::error(
      request.id,
      -32053,
      format!(
        "Plugin `{}` is already installed",
        candidate_plugin.display_name
      ),
    );
  }
  let installed_plugin = match install_plugin_bundle(&source_path, &context.plugin_install_root) {
    Ok(plugin) => plugin,
    Err(error) => return JsonRpcResponse::error(request.id, -32053, error.to_string()),
  };

  if let Err(error) = context.refresh_plugins() {
    return JsonRpcResponse::error(request.id, -32010, error.to_string());
  }

  let refreshed_plugin = context
    .plugins
    .iter()
    .find(|plugin| plugin.id == installed_plugin.id)
    .cloned()
    .unwrap_or(installed_plugin);

  JsonRpcResponse::success(
    request.id,
    &PluginInstallResult {
      plugin: to_protocol_plugin(refreshed_plugin),
    },
  )
}

fn handle_plugin_remove(context: &mut RuntimeContext, request: JsonRpcRequest) -> JsonRpcResponse {
  let params = match parse_required_params::<PluginRemoveParams>(&request, "plugin/remove") {
    Ok(params) => params,
    Err(response) => return response,
  };

  let manifest_path = PathBuf::from(&params.manifest_path);
  let removed_plugin =
    match remove_local_plugin_bundle(&manifest_path, &context.plugin_install_root) {
      Ok(plugin) => plugin,
      Err(error) => return JsonRpcResponse::error(request.id, -32054, error.to_string()),
    };

  if let Err(error) = context.delete_plugin_state(&removed_plugin.plugin_id) {
    return JsonRpcResponse::error(request.id, -32010, error.to_string());
  }
  if let Err(error) = context.refresh_plugins() {
    return JsonRpcResponse::error(request.id, -32010, error.to_string());
  }

  JsonRpcResponse::success(
    request.id,
    &PluginRemoveResult {
      plugin_id: removed_plugin.plugin_id,
      display_name: removed_plugin.display_name,
      removed_path: removed_plugin.removed_path,
    },
  )
}

fn handle_workspace_open(context: &mut RuntimeContext, request: JsonRpcRequest) -> JsonRpcResponse {
  let params = match parse_required_params::<WorkspaceOpenParams>(&request, "workspace/open") {
    Ok(params) => params,
    Err(response) => return response,
  };

  let workspace_path = PathBuf::from(params.path);
  if !workspace_path.is_dir() {
    return JsonRpcResponse::error(request.id, -32020, "Workspace path is not a directory");
  }

  let resolved_path = match fs::canonicalize(&workspace_path) {
    Ok(path) => path,
    Err(error) => {
      return JsonRpcResponse::error(
        request.id,
        -32021,
        format!("Failed to resolve workspace path: {error}"),
      )
    }
  };

  let workspace = WorkspaceSummary {
    root_path: resolved_path.display().to_string(),
    display_name: resolved_path
      .file_name()
      .map(|name| name.to_string_lossy().into_owned())
      .filter(|name| !name.is_empty())
      .unwrap_or_else(|| resolved_path.display().to_string()),
  };
  context.workspace = Some(workspace.clone());

  if let Err(error) = context.persist_workspace() {
    return JsonRpcResponse::error(request.id, -32010, error.to_string());
  }

  if let Err(error) = context.remember(MemoryEvent::WorkspaceOpened {
    display_name: workspace.display_name.clone(),
    root_path: workspace.root_path.clone(),
  }) {
    return JsonRpcResponse::error(request.id, -32011, error.to_string());
  }

  JsonRpcResponse::success(
    request.id,
    &WorkspaceOpenResult {
      workspace,
      thread_count: context.threads.len(),
    },
  )
}

fn handle_thread_read(context: &mut RuntimeContext, request: JsonRpcRequest) -> JsonRpcResponse {
  let params = match parse_required_params::<ThreadReadParams>(&request, "thread/read") {
    Ok(params) => params,
    Err(response) => return response,
  };

  let did_refresh = match refresh_active_turn_for_thread(context, &params.thread_id) {
    Ok(did_refresh) => did_refresh,
    Err(error) => {
      return JsonRpcResponse::error(request.id, -32010, error.to_string());
    }
  };

  if did_refresh {
    if let Err(error) = context.persist_runtime_state() {
      return JsonRpcResponse::error(request.id, -32010, error.to_string());
    }
  }

  let Some(thread) = context
    .threads
    .iter()
    .find(|thread| thread.summary.id == params.thread_id)
  else {
    return JsonRpcResponse::error(request.id, -32004, "Thread not found");
  };

  JsonRpcResponse::success(
    request.id,
    &ThreadReadResult {
      thread: thread.summary.clone(),
      items: thread.items.clone(),
      pending_approvals: approvals_for_thread(context, &thread.summary.id),
      active_turn_id: active_turn_id_for_thread(&context.active_turns, &thread.summary.id),
    },
  )
}

fn handle_workspace_search(
  context: &mut RuntimeContext,
  request: JsonRpcRequest,
) -> JsonRpcResponse {
  let params = match parse_required_params::<WorkspaceSearchParams>(&request, "workspace/search") {
    Ok(params) => params,
    Err(response) => return response,
  };

  let Some(workspace) = context.workspace.clone() else {
    return JsonRpcResponse::error(request.id, -32040, "Open a workspace before searching");
  };

  let max_results = params.max_results.unwrap_or(24).clamp(1, 100);
  let matches = match search_files(Path::new(&workspace.root_path), &params.query, max_results) {
    Ok(matches) => matches
      .into_iter()
      .map(|entry| WorkspaceSearchMatch {
        relative_path: entry.relative_path,
        line_number: entry.line_number,
        line: entry.line,
      })
      .collect::<Vec<_>>(),
    Err(error) => {
      return JsonRpcResponse::error(request.id, -32041, error.to_string());
    }
  };

  JsonRpcResponse::success(
    request.id,
    &WorkspaceSearchResult {
      query: params.query,
      workspace,
      matches,
    },
  )
}

fn handle_thread_start(context: &mut RuntimeContext, request: JsonRpcRequest) -> JsonRpcResponse {
  let params = match parse_required_params::<ThreadStartParams>(&request, "thread/start") {
    Ok(params) => params,
    Err(response) => return response,
  };

  let workspace = context.workspace.clone();
  let thread = ThreadSummary {
    id: format!("thread-{}", context.next_thread_number),
    title: params.title,
    status: "ready".to_string(),
    workspace: workspace.clone(),
  };
  context.next_thread_number += 1;
  let items = vec![TimelineItem {
    kind: "system".to_string(),
    title: "Thread Ready".to_string(),
    content: format!("{} is ready for local runtime messages.", thread.title),
    attributes: None,
  }];
  context.threads.push(StoredThread {
    summary: thread.clone(),
    turn_count: 0,
    items: items.clone(),
    workspace,
  });

  if let Err(error) = context.persist_runtime_state() {
    return JsonRpcResponse::error(request.id, -32010, error.to_string());
  }

  JsonRpcResponse::success(request.id, &ThreadStartResult { thread })
}

fn handle_turn_start(context: &mut RuntimeContext, request: JsonRpcRequest) -> JsonRpcResponse {
  let params = match parse_required_params::<TurnStartParams>(&request, "turn/start") {
    Ok(params) => params,
    Err(response) => return response,
  };

  match execute_turn_request(
    context,
    &params.thread_id,
    params.message.clone(),
    params.message,
    vec![],
  ) {
    Ok(result) => JsonRpcResponse::success(request.id, &result),
    Err((code, message)) => JsonRpcResponse::error(request.id, code, message),
  }
}

fn execute_turn_request(
  context: &mut RuntimeContext,
  requested_thread_id: &str,
  display_message: String,
  message: String,
  initial_items: Vec<TimelineItem>,
) -> std::result::Result<TurnStartResult, (i32, String)> {
  let current_workspace = context.workspace.clone();
  let model_runtime = context.model_runtime.clone();
  let memory_notes = context.memory_notes.clone();
  let permission_sources = granted_permission_sources(&context.plugins);
  let (thread_id, turn_id, items, active_turn_id, pending_active_turn) = {
    let Some(thread) = context
      .threads
      .iter_mut()
      .find(|thread| thread.summary.id == requested_thread_id)
    else {
      return Err((-32004, "Thread not found".to_string()));
    };

    thread.turn_count += 1;
    let turn_count = thread.turn_count;
    let thread_id = thread.summary.id.clone();
    let thread_title = thread.summary.title.clone();
    if thread.workspace.is_none() {
      thread.workspace = current_workspace.clone();
      thread.summary.workspace = thread.workspace.clone();
    }
    let workspace = thread.workspace.clone();
    let turn_id = format!("{thread_id}-turn-{turn_count}");
    let mut pending_active_turn = None;

    thread.summary.status = match &workspace {
      Some(workspace) => format!("{turn_count} turn(s) in {}", workspace.display_name),
      None => format!("{turn_count} turn(s)"),
    };

    let mut items = initial_items;
    items.push(TimelineItem {
      kind: "userMessage".to_string(),
      title: "User".to_string(),
      content: display_message.clone(),
      attributes: None,
    });

    if let Some(workspace) = workspace {
      let workspace_root = Path::new(&workspace.root_path);

      if let Some(write_intent) = infer_write_intent(&message) {
        items.push(build_plan_item(
          &context.model_runtime,
          &memory_notes,
          &message,
          Some(&workspace),
          if permission_is_granted(&permission_sources, "file.write") {
            format!(
              "Request approval before writing {} in {}.",
              write_intent.relative_path, workspace.display_name
            )
          } else {
            format!(
              "Check plugin permissions before writing {} in {}.",
              write_intent.relative_path, workspace.display_name
            )
          },
        ));
        if !permission_is_granted(&permission_sources, "file.write") {
          items.extend(build_permission_denied_items(
            &permission_sources,
            "file.write",
            "prepare a file write",
            &workspace.display_name,
            HashMap::from([(
              "relativePath".to_string(),
              write_intent.relative_path.clone(),
            )]),
          ));
        } else {
          let approval_id = format!("approval-{}", context.next_approval_number);
          context.next_approval_number += 1;

          let approval = PendingApproval {
            id: approval_id.clone(),
            thread_id: thread_id.clone(),
            action: "write_file".to_string(),
            title: format!("Write {}", write_intent.relative_path),
            relative_path: write_intent.relative_path.clone(),
            content: Some(write_intent.content.clone()),
            command: None,
          };
          context
            .pending_approvals
            .insert(approval_id.clone(), approval.clone());

          items.push(TimelineItem {
            kind: "toolStart".to_string(),
            title: "generate_diff".to_string(),
            content: write_intent.relative_path.clone(),
            attributes: None,
          });
          match generate_diff(
            workspace_root,
            &write_intent.relative_path,
            &write_intent.content,
          ) {
            Ok(diff) => {
              items.push(TimelineItem {
                kind: "diffArtifact".to_string(),
                title: "Diff Preview".to_string(),
                content: diff,
                attributes: Some(HashMap::from([
                  ("action".to_string(), "write_file".to_string()),
                  (
                    "relativePath".to_string(),
                    write_intent.relative_path.clone(),
                  ),
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
              write_intent.relative_path, workspace.display_name
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
              write_intent.relative_path
            ),
            attributes: None,
          });
        }
      } else if let Some(shell_command) = infer_shell_command(&message) {
        items.push(build_plan_item(
          &context.model_runtime,
          &memory_notes,
          &message,
          Some(&workspace),
          if permission_is_granted(&permission_sources, "shell.exec") {
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
        if !permission_is_granted(&permission_sources, "shell.exec") {
          items.extend(build_permission_denied_items(
            &permission_sources,
            "shell.exec",
            "run a shell command",
            &workspace.display_name,
            HashMap::from([("command".to_string(), shell_command.clone())]),
          ));
        } else {
          let approval_id = format!("approval-{}", context.next_approval_number);
          context.next_approval_number += 1;
          let sandbox = shell_sandbox_summary(Path::new(&workspace.root_path));

          let approval = PendingApproval {
            id: approval_id.clone(),
            thread_id: thread_id.clone(),
            action: "run_shell".to_string(),
            title: "Run Shell Command".to_string(),
            relative_path: ".".to_string(),
            content: None,
            command: Some(shell_command.clone()),
          };
          context
            .pending_approvals
            .insert(approval_id.clone(), approval.clone());

          items.push(TimelineItem {
            kind: "approvalRequested".to_string(),
            title: "Approval Requested".to_string(),
            content: format!(
              "Pith wants to run this shell command in {}:\n{}\n\n{}",
              workspace.display_name,
              shell_command,
              sandbox.display_line()
            ),
            attributes: Some({
              let mut attributes = sandbox.attributes();
              attributes.extend(HashMap::from([
                ("approvalId".to_string(), approval.id.clone()),
                ("action".to_string(), approval.action.clone()),
                ("command".to_string(), shell_command),
              ]));
              attributes
            }),
          });
          items.push(TimelineItem {
            kind: "assistantMessage".to_string(),
            title: "Assistant".to_string(),
            content: "Pith is waiting for your approval before running the shell command."
              .to_string(),
            attributes: None,
          });
        }
      } else if let Some(relative_path) = infer_requested_file_path(&message, workspace_root) {
        items.push(build_plan_item(
          &context.model_runtime,
          &memory_notes,
          &message,
          Some(&workspace),
          if permission_is_granted(&permission_sources, "file.read") {
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
        if !permission_is_granted(&permission_sources, "file.read") {
          items.extend(build_permission_denied_items(
            &permission_sources,
            "file.read",
            "inspect a file",
            &workspace.display_name,
            HashMap::from([("relativePath".to_string(), relative_path.clone())]),
          ));
        } else {
          items.push(TimelineItem {
            kind: "toolStart".to_string(),
            title: "read_file".to_string(),
            content: relative_path.clone(),
            attributes: None,
          });

          match read_file(workspace_root, &relative_path, 4096) {
            Ok(result) => {
              items.push(TimelineItem {
                kind: "toolResult".to_string(),
                title: "read_file result".to_string(),
                content: format_file_result(&result),
                attributes: None,
              });
              let (summary, summary_attributes) = summarize_file_result(
                &model_runtime,
                &memory_notes,
                &thread_title,
                &workspace.display_name,
                &result,
              );
              pending_active_turn = start_streaming_assistant_turn(
                &thread_id,
                &turn_id,
                &mut items,
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
      } else if let Some(search_query) = infer_search_query(&message) {
        items.push(build_plan_item(
          &context.model_runtime,
          &memory_notes,
          &message,
          Some(&workspace),
          if permission_is_granted(&permission_sources, "file.read") {
            format!(
              "Search {} for matches to \"{}\" with the built-in search_files tool.",
              workspace.display_name, search_query
            )
          } else {
            format!(
              "Check plugin permissions before searching {} for \"{}\".",
              workspace.display_name, search_query
            )
          },
        ));
        if !permission_is_granted(&permission_sources, "file.read") {
          items.extend(build_permission_denied_items(
            &permission_sources,
            "file.read",
            "search files",
            &workspace.display_name,
            HashMap::from([("query".to_string(), search_query.clone())]),
          ));
        } else {
          items.push(TimelineItem {
            kind: "toolStart".to_string(),
            title: "search_files".to_string(),
            content: search_query.clone(),
            attributes: None,
          });

          match search_files(workspace_root, &search_query, 12) {
            Ok(matches) => {
              items.push(TimelineItem {
                kind: "toolResult".to_string(),
                title: "search_files result".to_string(),
                content: format_search_result(&search_query, &matches),
                attributes: None,
              });
              let (summary, summary_attributes) = summarize_search_result(
                &model_runtime,
                &memory_notes,
                &thread_title,
                &workspace.display_name,
                &search_query,
                &matches,
              );
              pending_active_turn = start_streaming_assistant_turn(
                &thread_id,
                &turn_id,
                &mut items,
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
      } else {
        items.push(build_plan_item(
          &context.model_runtime,
          &memory_notes,
          &message,
          Some(&workspace),
          if permission_is_granted(&permission_sources, "file.read") {
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
        if !permission_is_granted(&permission_sources, "file.read") {
          items.extend(build_permission_denied_items(
            &permission_sources,
            "file.read",
            "inspect the workspace",
            &workspace.display_name,
            HashMap::new(),
          ));
        } else {
          items.push(TimelineItem {
            kind: "toolStart".to_string(),
            title: "list_directory".to_string(),
            content: ".".to_string(),
            attributes: None,
          });

          match list_directory(workspace_root, None, 24) {
            Ok(entries) => {
              items.push(TimelineItem {
                kind: "toolResult".to_string(),
                title: "list_directory result".to_string(),
                content: format_directory_result(&entries),
                attributes: None,
              });
              let (summary, summary_attributes) = summarize_directory_result(
                &model_runtime,
                &memory_notes,
                &thread_title,
                &workspace.display_name,
                &entries,
              );
              pending_active_turn = start_streaming_assistant_turn(
                &thread_id,
                &turn_id,
                &mut items,
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
      }
    } else {
      items.push(build_plan_item(
        &context.model_runtime,
        &memory_notes,
        &message,
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
          thread_title
        ),
        attributes: None,
      });
    }

    let active_turn_id = pending_active_turn.as_ref().map(|turn| turn.id.clone());
    if active_turn_id.is_some() {
      thread.summary.status = "Streaming assistant response".to_string();
    } else if !thread.summary.status.contains("approval") {
      thread.summary.status = "Ready".to_string();
    }

    thread.items.extend(items.clone());

    (
      thread_id,
      turn_id,
      items,
      active_turn_id,
      pending_active_turn,
    )
  };

  if let Some(active_turn) = pending_active_turn {
    context
      .active_turns
      .insert(active_turn.id.clone(), active_turn);
  }

  if let Err(error) = context.persist_runtime_state() {
    return Err((-32010, error.to_string()));
  }

  if active_turn_id.is_none() {
    if let Err(error) = refresh_thread_summary_note(context, &thread_id) {
      return Err((-32012, error.to_string()));
    }
  }

  let pending_approvals = approvals_for_thread(context, &thread_id);

  Ok(TurnStartResult {
    turn_id,
    thread_id,
    items,
    pending_approvals,
    active_turn_id,
  })
}

fn capture_plugin_hook_memory(
  context: &mut RuntimeContext,
  workspace: &WorkspaceSummary,
  capture: &PluginHookMemoryCapture,
) -> Result<TimelineItem> {
  let Some(note_title) = capture.hook.memory_note_title.as_ref() else {
    return Ok(TimelineItem {
      kind: "system".to_string(),
      title: "Plugin Hook Memory Skipped".to_string(),
      content: format!(
        "{} did not declare a memory note title.",
        capture.hook.title
      ),
      attributes: Some(HashMap::from([(
        "hookId".to_string(),
        capture.hook.hook_id.clone(),
      )])),
    });
  };
  let note_source = capture
    .hook
    .memory_note_source
    .clone()
    .unwrap_or_else(|| format!("plugin.{}", capture.hook.plugin_id));
  let note = context.create_memory_note(
    note_title.clone(),
    build_plugin_hook_memory_body(workspace, capture),
    workspace.display_name.clone(),
    note_source,
    plugin_hook_memory_tags(&capture.hook),
  )?;

  Ok(TimelineItem {
    kind: "system".to_string(),
    title: "Hook Memory Note Saved".to_string(),
    content: format!(
      "Saved workspace memory note \"{}\" from {}.",
      note.title, capture.hook.title
    ),
    attributes: Some(HashMap::from([
      ("memoryNoteId".to_string(), note.id),
      ("memoryNoteTitle".to_string(), note.title),
      ("memoryScope".to_string(), note.scope),
      ("pluginId".to_string(), capture.hook.plugin_id.clone()),
      ("hookId".to_string(), capture.hook.hook_id.clone()),
    ])),
  })
}

fn handle_approval_respond(
  context: &mut RuntimeContext,
  request: JsonRpcRequest,
) -> JsonRpcResponse {
  let params = match parse_required_params::<ApprovalRespondParams>(&request, "approval/respond") {
    Ok(params) => params,
    Err(response) => return response,
  };

  let decision = params.decision.to_lowercase();
  if decision != "approved" && decision != "denied" {
    return JsonRpcResponse::error(
      request.id,
      -32602,
      "approval/respond decision must be approved or denied",
    );
  }

  let Some(approval) = context.pending_approvals.get(&params.approval_id).cloned() else {
    return JsonRpcResponse::error(request.id, -32030, "Approval request not found");
  };

  let current_workspace = context.workspace.clone();
  let model_runtime = context.model_runtime.clone();
  let memory_notes = context.memory_notes.clone();
  let permission_sources = granted_permission_sources(&context.plugins);

  let Some(thread) = context
    .threads
    .iter_mut()
    .find(|thread| thread.summary.id == approval.thread_id)
  else {
    return JsonRpcResponse::error(request.id, -32004, "Thread not found");
  };
  if thread.workspace.is_none() {
    thread.workspace = current_workspace;
    thread.summary.workspace = thread.workspace.clone();
  }
  let workspace = match thread.workspace.clone() {
    Some(workspace) => workspace,
    None => {
      return JsonRpcResponse::error(
        request.id,
        -32031,
        "Open a workspace for this thread before resolving approvals",
      )
    }
  };

  context.pending_approvals.remove(&params.approval_id);

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
              build_shell_completed_hook_items(&context.plugins, &workspace, &command, &result);
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

  thread.items.extend(items.clone());

  if let Err(error) = context.persist_resolved_approval(&approval, &decision) {
    return JsonRpcResponse::error(request.id, -32010, error.to_string());
  }

  if let Err(error) = context.persist_runtime_state() {
    return JsonRpcResponse::error(request.id, -32010, error.to_string());
  }

  if let Some(memory_event) = memory_event {
    if let Err(error) = context.remember(memory_event) {
      return JsonRpcResponse::error(request.id, -32012, error.to_string());
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
      return JsonRpcResponse::error(request.id, -32010, error.to_string());
    }
  }

  if let Err(error) = refresh_thread_summary_note(context, &approval.thread_id) {
    return JsonRpcResponse::error(request.id, -32012, error.to_string());
  }

  let pending_approvals = approvals_for_thread(context, &approval.thread_id);

  JsonRpcResponse::success(
    request.id,
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

fn refresh_active_turn_for_thread(context: &mut RuntimeContext, thread_id: &str) -> Result<bool> {
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

fn approvals_for_thread(context: &RuntimeContext, thread_id: &str) -> Vec<ApprovalRequest> {
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
