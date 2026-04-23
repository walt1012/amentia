use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::Result;
use cavell_memory::{retrieve_relevant_notes, MemoryEvent, MemoryManager, MemoryNote};
use cavell_model_runtime::{GenerateRequest, LocalModelRuntime, ModelHealth, ModelRole};
use cavell_plugin_host::{default_plugin_root, discover_plugins, PluginCatalogEntry};
use cavell_protocol::{
  methods, ApprovalRequest, ApprovalRespondParams, ApprovalRespondResult, HealthPingResult,
  InitializeParams, InitializeResult, JsonRpcNotification, JsonRpcRequest, JsonRpcResponse,
  MemoryListResult, MemoryNoteSummary, MemoryStatusResult, ModelHealthResult, PluginListResult,
  PluginSummary as ProtocolPluginSummary, ServerCapabilities, ServerInfo, ThreadListResult,
  ThreadReadParams, ThreadReadResult, ThreadStartParams, ThreadStartResult, ThreadSummary,
  ThreadUpdatedNotificationParams, TimelineItem, TurnCancelParams, TurnCancelResult,
  TurnStartParams, TurnStartResult, WorkspaceCurrentResult, WorkspaceOpenParams,
  WorkspaceOpenResult, WorkspaceSummary,
};
use cavell_storage::{FileThreadStore, StoredApprovalRecord, StoredThreadRecord};
use cavell_tools::{
  generate_diff, list_directory, read_file, run_shell, search_files, write_file, DirectoryEntry,
  ReadFileResult, SearchMatch, ShellCommandResult,
};

#[derive(Debug, Clone)]
struct StoredThread {
  summary: ThreadSummary,
  turn_count: usize,
  items: Vec<TimelineItem>,
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
struct WriteIntent {
  relative_path: String,
  content: String,
}

#[derive(Debug, Clone)]
struct ActiveTurn {
  id: String,
  thread_id: String,
  full_content: String,
  emitted_chars: usize,
  total_chars: usize,
  started_at: Instant,
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
  plugins: Vec<PluginCatalogEntry>,
  pending_approvals: HashMap<String, PendingApproval>,
  active_turns: HashMap<String, ActiveTurn>,
  next_thread_number: usize,
  next_approval_number: usize,
}

impl RuntimeContext {
  pub fn new() -> Result<Self> {
    let store = FileThreadStore::new_default()?;
    let persisted_threads = store.load_threads()?;
    let persisted_workspace = store.load_workspace()?;
    let persisted_pending_approvals = store.load_pending_approvals()?;
    let persisted_memory_notes = store.load_memory_notes(128)?;
    let plugins = load_plugin_catalog()?;
    let next_thread_number = persisted_threads.len() + 1;
    let next_approval_number = store.next_approval_sequence()?;
    let next_memory_number = store.next_memory_sequence()?;

    Ok(Self {
      server_name: "cavell-runtime".to_string(),
      server_version: env!("CARGO_PKG_VERSION").to_string(),
      model_runtime: LocalModelRuntime::new_default(),
      memory_manager: MemoryManager::new(next_memory_number),
      store: Some(store),
      memory_notes: persisted_memory_notes,
      threads: persisted_threads
        .into_iter()
        .map(|thread| StoredThread {
          summary: thread.summary,
          turn_count: thread.turn_count,
          items: thread.items,
        })
        .collect(),
      workspace: persisted_workspace,
      plugins,
      pending_approvals: persisted_pending_approvals
        .into_iter()
        .map(|approval| {
          (
            approval.id.clone(),
            PendingApproval {
              id: approval.id,
              thread_id: approval.thread_id,
              action: approval.action,
              title: approval.title,
              relative_path: approval.relative_path,
              content: approval.content,
              command: approval.command,
            },
          )
        })
        .collect(),
      active_turns: HashMap::new(),
      next_thread_number,
      next_approval_number,
    })
  }

  pub fn new_in_memory() -> Self {
    Self {
      server_name: "cavell-runtime".to_string(),
      server_version: env!("CARGO_PKG_VERSION").to_string(),
      model_runtime: LocalModelRuntime::new_default(),
      memory_manager: MemoryManager::new(1),
      store: None,
      memory_notes: vec![],
      threads: vec![],
      workspace: None,
      plugins: load_plugin_catalog().unwrap_or_default(),
      pending_approvals: HashMap::new(),
      active_turns: HashMap::new(),
      next_thread_number: 1,
      next_approval_number: 1,
    }
  }

  fn persist_threads(&self) -> Result<()> {
    let Some(store) = &self.store else {
      return Ok(());
    };

    let threads = self
      .threads
      .iter()
      .map(|thread| StoredThreadRecord {
        summary: thread.summary.clone(),
        turn_count: thread.turn_count,
        items: thread.items.clone(),
      })
      .collect::<Vec<_>>();

    store.save_threads(&threads)
  }

  fn persist_pending_approvals(&self) -> Result<()> {
    let Some(store) = &self.store else {
      return Ok(());
    };

    let approvals = self
      .pending_approvals
      .values()
      .cloned()
      .map(stored_approval_record)
      .collect::<Vec<_>>();

    store.save_pending_approvals(&approvals)
  }

  fn persist_runtime_state(&self) -> Result<()> {
    self.persist_threads()?;
    self.persist_pending_approvals()
  }

  fn persist_memory_note(&self, note: &MemoryNote) -> Result<()> {
    let Some(store) = &self.store else {
      return Ok(());
    };

    store.save_memory_note(note)
  }

  fn persist_workspace(&self) -> Result<()> {
    let Some(store) = &self.store else {
      return Ok(());
    };
    let Some(workspace) = &self.workspace else {
      return Ok(());
    };

    store.save_workspace(workspace)
  }

  fn persist_resolved_approval(&self, approval: &PendingApproval, decision: &str) -> Result<()> {
    let Some(store) = &self.store else {
      return Ok(());
    };

    store.resolve_approval(&stored_approval_record(approval.clone()), decision)
  }

  fn remember(&mut self, event: MemoryEvent) -> Result<MemoryNote> {
    let note = self
      .memory_manager
      .record_event(&mut self.memory_notes, event);
    self.persist_memory_note(&note)?;
    Ok(note)
  }
}

impl Default for RuntimeContext {
  fn default() -> Self {
    Self::new_in_memory()
  }
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
    methods::MODEL_HEALTH => JsonRpcResponse::success(
      request.id,
      &to_protocol_model_health(context.model_runtime.health()),
    ),
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
    methods::WORKSPACE_CURRENT => JsonRpcResponse::success(
      request.id,
      &WorkspaceCurrentResult {
        workspace: context.workspace.clone(),
      },
    ),
    methods::WORKSPACE_OPEN => handle_workspace_open(context, request),
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
    if let Some(params) = advance_active_turn(context, &turn_id) {
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

fn to_protocol_model_health(health: ModelHealth) -> ModelHealthResult {
  ModelHealthResult {
    pack_id: health.pack_id,
    display_name: health.display_name,
    backend: health.backend,
    status: health.status,
    detail: health.detail,
    source: health.source,
    binary_path: health.binary_path,
    model_path: health.model_path,
    manifest_path: health.manifest_path,
    metrics: health.metrics,
  }
}

fn to_protocol_memory_note(note: MemoryNote) -> MemoryNoteSummary {
  MemoryNoteSummary {
    id: note.id,
    title: note.title,
    body: note.body,
    scope: note.scope,
    source: note.source,
    created_at: note.created_at,
    tags: note.tags,
  }
}

fn to_protocol_memory_status(status: cavell_memory::MemoryStatus) -> MemoryStatusResult {
  MemoryStatusResult {
    note_count: status.note_count,
    latest_title: status.latest_title,
    summary: status.summary,
  }
}

fn to_protocol_plugin(plugin: PluginCatalogEntry) -> ProtocolPluginSummary {
  ProtocolPluginSummary {
    id: plugin.id,
    name: plugin.name,
    version: plugin.version,
    display_name: plugin.display_name,
    description: plugin.description,
    author_name: plugin.author_name,
    enabled: plugin.enabled,
    default_enabled: plugin.default_enabled,
    capabilities: plugin.capabilities,
    permissions: plugin.permissions,
    manifest_path: plugin.manifest_path,
    provenance: plugin.provenance,
  }
}

fn handle_initialize(context: &RuntimeContext, request: JsonRpcRequest) -> JsonRpcResponse {
  let params = match request.params {
    Some(value) => match serde_json::from_value::<InitializeParams>(value) {
      Ok(params) => params,
      Err(error) => {
        return JsonRpcResponse::error(
          request.id,
          -32602,
          format!("Invalid initialize params: {error}"),
        )
      }
    },
    None => {
      return JsonRpcResponse::error(request.id, -32602, "Missing initialize params");
    }
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
      },
    },
  )
}

fn load_plugin_catalog() -> Result<Vec<PluginCatalogEntry>> {
  let Some(plugin_root) = default_plugin_root() else {
    return Ok(vec![]);
  };

  discover_plugins(&plugin_root)
}

fn handle_workspace_open(context: &mut RuntimeContext, request: JsonRpcRequest) -> JsonRpcResponse {
  let params = match request.params {
    Some(value) => match serde_json::from_value::<WorkspaceOpenParams>(value) {
      Ok(params) => params,
      Err(error) => {
        return JsonRpcResponse::error(
          request.id,
          -32602,
          format!("Invalid workspace/open params: {error}"),
        )
      }
    },
    None => {
      return JsonRpcResponse::error(request.id, -32602, "Missing workspace/open params");
    }
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
  let params = match request.params {
    Some(value) => match serde_json::from_value::<ThreadReadParams>(value) {
      Ok(params) => params,
      Err(error) => {
        return JsonRpcResponse::error(
          request.id,
          -32602,
          format!("Invalid thread/read params: {error}"),
        )
      }
    },
    None => {
      return JsonRpcResponse::error(request.id, -32602, "Missing thread/read params");
    }
  };

  refresh_active_turn_for_thread(context, &params.thread_id);

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
      active_turn_id: active_turn_id_for_thread(context, &thread.summary.id),
    },
  )
}

fn handle_thread_start(context: &mut RuntimeContext, request: JsonRpcRequest) -> JsonRpcResponse {
  let params = match request.params {
    Some(value) => match serde_json::from_value::<ThreadStartParams>(value) {
      Ok(params) => params,
      Err(error) => {
        return JsonRpcResponse::error(
          request.id,
          -32602,
          format!("Invalid thread/start params: {error}"),
        )
      }
    },
    None => {
      return JsonRpcResponse::error(request.id, -32602, "Missing thread/start params");
    }
  };

  let thread = ThreadSummary {
    id: format!("thread-{}", context.next_thread_number),
    title: params.title,
    status: "ready".to_string(),
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
  });

  if let Err(error) = context.persist_runtime_state() {
    return JsonRpcResponse::error(request.id, -32010, error.to_string());
  }

  JsonRpcResponse::success(request.id, &ThreadStartResult { thread })
}

fn handle_turn_start(context: &mut RuntimeContext, request: JsonRpcRequest) -> JsonRpcResponse {
  let params = match request.params {
    Some(value) => match serde_json::from_value::<TurnStartParams>(value) {
      Ok(params) => params,
      Err(error) => {
        return JsonRpcResponse::error(
          request.id,
          -32602,
          format!("Invalid turn/start params: {error}"),
        )
      }
    },
    None => {
      return JsonRpcResponse::error(request.id, -32602, "Missing turn/start params");
    }
  };

  let workspace = context.workspace.clone();
  let model_runtime = context.model_runtime.clone();
  let memory_notes = context.memory_notes.clone();
  let (thread_id, turn_id, items, active_turn_id, pending_active_turn) = {
    let Some(thread) = context
      .threads
      .iter_mut()
      .find(|thread| thread.summary.id == params.thread_id)
    else {
      return JsonRpcResponse::error(request.id, -32004, "Thread not found");
    };

    thread.turn_count += 1;
    let turn_count = thread.turn_count;
    let thread_id = thread.summary.id.clone();
    let thread_title = thread.summary.title.clone();
    let message = params.message;
    let turn_id = format!("{thread_id}-turn-{turn_count}");
    let mut pending_active_turn = None;

    thread.summary.status = match &workspace {
      Some(workspace) => format!("{turn_count} turn(s) in {}", workspace.display_name),
      None => format!("{turn_count} turn(s)"),
    };

    let mut items = vec![TimelineItem {
      kind: "userMessage".to_string(),
      title: "User".to_string(),
      content: message.clone(),
      attributes: None,
    }];

    if let Some(workspace) = workspace {
      let workspace_root = Path::new(&workspace.root_path);

      if let Some(write_intent) = infer_write_intent(&message) {
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

        items.push(build_plan_item(
          &context.model_runtime,
          &memory_notes,
          &message,
          Some(&workspace),
          format!(
            "Request approval before writing {} in {}.",
            write_intent.relative_path, workspace.display_name
          ),
        ));
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
            "Cavell wants to write {} in {}.",
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
            "Cavell prepared a write for {} and is waiting for your approval.",
            write_intent.relative_path
          ),
          attributes: None,
        });
      } else if let Some(shell_command) = infer_shell_command(&message) {
        let approval_id = format!("approval-{}", context.next_approval_number);
        context.next_approval_number += 1;

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

        items.push(build_plan_item(
          &context.model_runtime,
          &memory_notes,
          &message,
          Some(&workspace),
          format!(
            "Request approval before running a shell command in {}.",
            workspace.display_name
          ),
        ));
        items.push(TimelineItem {
          kind: "approvalRequested".to_string(),
          title: "Approval Requested".to_string(),
          content: format!(
            "Cavell wants to run this shell command in {}:\n{}",
            workspace.display_name, shell_command
          ),
          attributes: Some(HashMap::from([
            ("approvalId".to_string(), approval.id.clone()),
            ("action".to_string(), approval.action.clone()),
            ("command".to_string(), shell_command),
          ])),
        });
        items.push(TimelineItem {
          kind: "assistantMessage".to_string(),
          title: "Assistant".to_string(),
          content: "Cavell is waiting for your approval before running the shell command."
            .to_string(),
          attributes: None,
        });
      } else if let Some(relative_path) = infer_requested_file_path(&message, workspace_root) {
        items.push(build_plan_item(
          &context.model_runtime,
          &memory_notes,
          &message,
          Some(&workspace),
          format!(
            "Inspect {} in {} with the built-in read_file tool.",
            relative_path, workspace.display_name
          ),
        ));
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
            pending_active_turn = maybe_start_streaming_assistant_turn(
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
                "Cavell could not inspect that file in {}. Try another path inside the workspace.",
                workspace.display_name
              ),
              attributes: None,
            });
          }
        }
      } else if let Some(search_query) = infer_search_query(&message) {
        items.push(build_plan_item(
          &context.model_runtime,
          &memory_notes,
          &message,
          Some(&workspace),
          format!(
            "Search {} for matches to \"{}\" with the built-in search_files tool.",
            workspace.display_name, search_query
          ),
        ));
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
            pending_active_turn = maybe_start_streaming_assistant_turn(
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
                "Cavell could not search {} yet. Try a shorter query or re-open the workspace.",
                workspace.display_name
              ),
              attributes: None,
            });
          }
        }
      } else {
        items.push(build_plan_item(
          &context.model_runtime,
          &memory_notes,
          &message,
          Some(&workspace),
          format!(
            "Inspect the root of {} with the built-in list_directory tool.",
            workspace.display_name
          ),
        ));
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
            pending_active_turn = maybe_start_streaming_assistant_turn(
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
                "Cavell could not inspect the root of {} yet. Re-open the workspace and try again.",
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
        None,
        "Wait for a workspace before running filesystem tools.".to_string(),
      ));
      items.push(TimelineItem {
        kind: "warning".to_string(),
        title: "Workspace Required".to_string(),
        content: "Open a workspace before asking Cavell to inspect files.".to_string(),
        attributes: None,
      });
      items.push(TimelineItem {
        kind: "assistantMessage".to_string(),
        title: "Assistant".to_string(),
        content: format!(
          "Cavell received your message in {}, but Milestone 1 tools need an opened workspace first.",
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
    return JsonRpcResponse::error(request.id, -32010, error.to_string());
  }

  let pending_approvals = approvals_for_thread(context, &thread_id);

  JsonRpcResponse::success(
    request.id,
    &TurnStartResult {
      turn_id,
      thread_id,
      items,
      pending_approvals,
      active_turn_id,
    },
  )
}

fn handle_approval_respond(
  context: &mut RuntimeContext,
  request: JsonRpcRequest,
) -> JsonRpcResponse {
  let params = match request.params {
    Some(value) => match serde_json::from_value::<ApprovalRespondParams>(value) {
      Ok(params) => params,
      Err(error) => {
        return JsonRpcResponse::error(
          request.id,
          -32602,
          format!("Invalid approval/respond params: {error}"),
        )
      }
    },
    None => {
      return JsonRpcResponse::error(request.id, -32602, "Missing approval/respond params");
    }
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

  let workspace = match context.workspace.clone() {
    Some(workspace) => workspace,
    None => {
      return JsonRpcResponse::error(
        request.id,
        -32031,
        "Open a workspace before resolving approvals",
      )
    }
  };
  let model_runtime = context.model_runtime.clone();
  let memory_notes = context.memory_notes.clone();

  let Some(thread) = context
    .threads
    .iter_mut()
    .find(|thread| thread.summary.id == approval.thread_id)
  else {
    return JsonRpcResponse::error(request.id, -32004, "Thread not found");
  };

  context.pending_approvals.remove(&params.approval_id);

  let mut items = vec![];
  let mut memory_event = None;
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
              "Cavell wrote {} in {} after your approval.",
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
    } else if approval.action == "run_shell" {
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
            attributes: None,
          });
          items.push(TimelineItem {
            kind: "assistantMessage".to_string(),
            title: "Assistant".to_string(),
            content: summary,
            attributes: Some(summary_attributes),
          });
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
  } else {
    memory_event = Some(MemoryEvent::ApprovalDenied {
      title: approval.title.clone(),
      action: approval.action.clone(),
    });
    let (summary, summary_attributes) = summarize_denied_approval(
      &model_runtime,
      &memory_notes,
      &workspace.display_name,
      &approval,
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
  let params = match request.params {
    Some(value) => match serde_json::from_value::<TurnCancelParams>(value) {
      Ok(params) => params,
      Err(error) => {
        return JsonRpcResponse::error(
          request.id,
          -32602,
          format!("Invalid turn/cancel params: {error}"),
        )
      }
    },
    None => {
      return JsonRpcResponse::error(request.id, -32602, "Missing turn/cancel params");
    }
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
    thread,
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
      content: "Cavell stopped the active response at your request.".to_string(),
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

  JsonRpcResponse::success(
    request.id,
    &TurnCancelResult {
      turn_id: params.turn_id,
      thread_id: active_turn_snapshot.thread_id,
      items,
      active_turn_id: active_turn_id_for_thread(context, &cancelled_thread_id),
    },
  )
}

fn infer_requested_file_path(message: &str, workspace_root: &Path) -> Option<String> {
  let common_files = ["README.md", "Cargo.toml", "Package.swift"];
  let lowercased_message = message.to_lowercase();

  for candidate in common_files {
    if lowercased_message.contains(&candidate.to_lowercase())
      && workspace_root.join(candidate).is_file()
    {
      return Some(candidate.to_string());
    }
  }

  let punctuation: &[char] = &['`', '"', '\'', ',', ';', ':', '(', ')', '[', ']', '{', '}'];
  for token in message.split_whitespace() {
    let candidate = token.trim_matches(punctuation);
    if candidate.is_empty() || (!candidate.contains('/') && !candidate.contains('.')) {
      continue;
    }

    if workspace_root.join(candidate).is_file() {
      return Some(candidate.replace('\\', "/"));
    }
  }

  None
}

fn infer_write_intent(message: &str) -> Option<WriteIntent> {
  let trimmed = message.trim();
  let lowercased_message = trimmed.to_lowercase();

  for prefix in ["write ", "create ", "update "] {
    if lowercased_message.starts_with(prefix) {
      let remainder = &trimmed[prefix.len()..];
      let (path, content) = remainder.split_once(':')?;
      let relative_path = path
        .trim()
        .trim_matches(&['"', '\'', '`'][..])
        .replace('\\', "/");
      let content = content.trim().to_string();

      if relative_path.is_empty() || content.is_empty() {
        return None;
      }

      return Some(WriteIntent {
        relative_path,
        content,
      });
    }
  }

  None
}

fn infer_shell_command(message: &str) -> Option<String> {
  let trimmed = message.trim();
  let lowercased_message = trimmed.to_lowercase();

  for prefix in ["run shell:", "shell:", "run command:"] {
    if lowercased_message.starts_with(prefix) {
      let command = trimmed[prefix.len()..].trim();
      if !command.is_empty() {
        return Some(command.to_string());
      }
    }
  }

  None
}

fn infer_search_query(message: &str) -> Option<String> {
  let trimmed = message.trim();
  let lowercased_message = trimmed.to_lowercase();

  for keyword in ["search for ", "find ", "search "] {
    if let Some(index) = lowercased_message.find(keyword) {
      let query = trimmed[index + keyword.len()..]
        .trim()
        .trim_matches(&['"', '\'', '.', '?', '!', '`'][..]);
      if !query.is_empty() {
        return Some(query.to_string());
      }
    }
  }

  if lowercased_message.contains("grep ") {
    let query = trimmed
      .split_once("grep ")
      .map(|(_, remainder)| remainder.trim())
      .unwrap_or_default()
      .trim_matches(&['"', '\'', '.', '?', '!', '`'][..]);
    if !query.is_empty() {
      return Some(query.to_string());
    }
  }

  None
}

fn maybe_start_streaming_assistant_turn(
  thread_id: &str,
  turn_id: &str,
  items: &mut Vec<TimelineItem>,
  full_content: String,
  mut attributes: HashMap<String, String>,
) -> Option<ActiveTurn> {
  let initial_chars = 48.min(full_content.chars().count());
  let total_chars = full_content.chars().count();
  let initial_content = take_characters(&full_content, initial_chars);
  let is_complete = initial_chars >= total_chars;
  let streaming_status = if is_complete {
    "completed"
  } else {
    "in_progress"
  };
  attributes.insert("turnId".to_string(), turn_id.to_string());
  attributes.insert("streamingStatus".to_string(), streaming_status.to_string());
  attributes.insert("streamedCharacters".to_string(), initial_chars.to_string());
  attributes.insert("totalCharacters".to_string(), total_chars.to_string());
  attributes.insert("responseRole".to_string(), "summarizer".to_string());

  items.push(TimelineItem {
    kind: "assistantMessage".to_string(),
    title: "Assistant".to_string(),
    content: initial_content,
    attributes: Some(attributes),
  });

  if is_complete {
    return None;
  }

  Some(ActiveTurn {
    id: turn_id.to_string(),
    thread_id: thread_id.to_string(),
    full_content,
    emitted_chars: initial_chars,
    total_chars,
    started_at: Instant::now(),
  })
}

fn refresh_active_turn_for_thread(context: &mut RuntimeContext, thread_id: &str) {
  let active_turn_ids = context
    .active_turns
    .values()
    .filter(|turn| turn.thread_id == thread_id)
    .map(|turn| turn.id.clone())
    .collect::<Vec<_>>();

  for turn_id in active_turn_ids {
    let _ = advance_active_turn(context, &turn_id);
  }
}

fn advance_active_turn(
  context: &mut RuntimeContext,
  turn_id: &str,
) -> Option<ThreadUpdatedNotificationParams> {
  let snapshot = context.active_turns.get(turn_id).cloned()?;
  let target_chars = compute_streamed_char_count(&snapshot).min(snapshot.total_chars);

  if target_chars <= snapshot.emitted_chars {
    return None;
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
    let thread = context
      .threads
      .iter_mut()
      .find(|thread| thread.summary.id == snapshot.thread_id)?;

    update_streaming_item(
      thread,
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
  } else if let Some(active_turn) = context.active_turns.get_mut(turn_id) {
    active_turn.emitted_chars = target_chars;
  }

  Some(ThreadUpdatedNotificationParams {
    thread: thread_snapshot.0,
    items: thread_snapshot.1,
    pending_approvals: approvals_for_thread(context, &thread_id),
    active_turn_id: active_turn_id_for_thread(context, &thread_id),
  })
}

fn compute_streamed_char_count(turn: &ActiveTurn) -> usize {
  let elapsed_steps = (turn.started_at.elapsed().as_millis() / 180) as usize;
  let base_chars = 48;
  let step_chars = 72;

  base_chars + elapsed_steps * step_chars
}

fn update_streaming_item(
  thread: &mut StoredThread,
  turn_id: &str,
  content: &str,
  streaming_status: &str,
  streamed_chars: usize,
  total_chars: usize,
) {
  let Some(item) = thread.items.iter_mut().rev().find(|item| {
    item.kind == "assistantMessage"
      && item
        .attributes
        .as_ref()
        .and_then(|attributes| attributes.get("turnId"))
        .map(|value| value == turn_id)
        .unwrap_or(false)
  }) else {
    return;
  };

  item.content = content.to_string();
  let mut attributes = item.attributes.clone().unwrap_or_default();
  attributes.insert("turnId".to_string(), turn_id.to_string());
  attributes.insert("streamingStatus".to_string(), streaming_status.to_string());
  attributes.insert("streamedCharacters".to_string(), streamed_chars.to_string());
  attributes.insert("totalCharacters".to_string(), total_chars.to_string());
  item.attributes = Some(attributes);
}

fn active_turn_id_for_thread(context: &RuntimeContext, thread_id: &str) -> Option<String> {
  context
    .active_turns
    .values()
    .find(|turn| turn.thread_id == thread_id)
    .map(|turn| turn.id.clone())
}

fn streaming_progress_label(streamed_chars: usize, total_chars: usize) -> String {
  if total_chars == 0 {
    return "0%".to_string();
  }

  let percentage = ((streamed_chars as f64 / total_chars as f64) * 100.0).round() as usize;
  format!("{}%", percentage.min(100))
}

fn build_plan_item(
  model_runtime: &LocalModelRuntime,
  memory_notes: &[MemoryNote],
  message: &str,
  workspace: Option<&WorkspaceSummary>,
  fallback: String,
) -> TimelineItem {
  let relevant_memory_notes = retrieve_memory_context(
    memory_notes,
    workspace.map(|entry| entry.display_name.as_str()),
    message,
  );
  let workspace_context = workspace
    .map(|workspace| {
      format!(
        "Workspace: {} at {}.",
        workspace.display_name, workspace.root_path
      )
    })
    .unwrap_or_else(|| "Workspace: unavailable.".to_string());
  let result = model_runtime.generate(GenerateRequest {
    role: ModelRole::Planner,
    prompt: format!(
      "You are the local planner for Cavell.\n{}\n{}\nUser request: {}\nWrite one concise English sentence describing the next action Cavell should take.",
      workspace_context,
      format_memory_prompt(&relevant_memory_notes),
      message
    ),
    fallback,
    max_tokens: 80,
  });
  let mut attributes = HashMap::from([
    ("responseRole".to_string(), "planner".to_string()),
    ("modelId".to_string(), result.model_id),
    ("modelBackend".to_string(), result.backend),
    ("modelStatus".to_string(), result.status),
  ]);
  if let Some(workspace) = workspace {
    attributes.insert(
      "workspaceDisplayName".to_string(),
      workspace.display_name.clone(),
    );
  }
  merge_memory_attributes(&mut attributes, &relevant_memory_notes);

  TimelineItem {
    kind: "plan".to_string(),
    title: "Plan".to_string(),
    content: result.text,
    attributes: Some(attributes),
  }
}

fn take_characters(content: &str, count: usize) -> String {
  content.chars().take(count).collect()
}

fn format_file_result(result: &ReadFileResult) -> String {
  if result.is_truncated {
    format!(
      "File: {}\n\n{}\n\n[output truncated at 4096 bytes]",
      result.relative_path, result.content
    )
  } else {
    format!("File: {}\n\n{}", result.relative_path, result.content)
  }
}

fn summarize_file_result(
  model_runtime: &LocalModelRuntime,
  memory_notes: &[MemoryNote],
  thread_title: &str,
  workspace_name: &str,
  result: &ReadFileResult,
) -> (String, HashMap<String, String>) {
  let relevant_memory_notes = retrieve_memory_context(
    memory_notes,
    Some(workspace_name),
    &format!("{thread_title} {}", result.relative_path),
  );
  let preview = result
    .content
    .lines()
    .find(|line| !line.trim().is_empty())
    .unwrap_or("The file is empty.");

  let fallback = format!(
    "Cavell inspected {} for {} in {}. First useful line: {}",
    result.relative_path, thread_title, workspace_name, preview
  );
  let prompt = format!(
    "You are Cavell, a concise local coding agent. Summarize a file inspection in one or two sentences.\nThread: {thread_title}\nWorkspace: {workspace_name}\n{}\nFile: {}\nPreview:\n{}",
    format_memory_prompt(&relevant_memory_notes),
    result.relative_path,
    result.content
  );

  generate_local_summary(model_runtime, prompt, fallback, &relevant_memory_notes)
}

fn format_directory_result(entries: &[DirectoryEntry]) -> String {
  if entries.is_empty() {
    return "The directory is empty.".to_string();
  }

  entries
    .iter()
    .map(|entry| format!("[{}] {}", entry.entry_type, entry.relative_path))
    .collect::<Vec<_>>()
    .join("\n")
}

fn format_search_result(query: &str, matches: &[SearchMatch]) -> String {
  if matches.is_empty() {
    return format!("No matches found for \"{}\".", query);
  }

  matches
    .iter()
    .map(|entry| {
      format!(
        "{}:{}: {}",
        entry.relative_path, entry.line_number, entry.line
      )
    })
    .collect::<Vec<_>>()
    .join("\n")
}

fn summarize_directory_result(
  model_runtime: &LocalModelRuntime,
  memory_notes: &[MemoryNote],
  thread_title: &str,
  workspace_name: &str,
  entries: &[DirectoryEntry],
) -> (String, HashMap<String, String>) {
  let relevant_memory_notes = retrieve_memory_context(
    memory_notes,
    Some(workspace_name),
    &format!("{thread_title} workspace root"),
  );
  if entries.is_empty() {
    return generate_local_summary(
      model_runtime,
      format!(
        "You are Cavell, a concise local coding agent. Summarize an empty workspace root inspection.\nThread: {thread_title}\nWorkspace: {workspace_name}\n{}",
        format_memory_prompt(&relevant_memory_notes)
      ),
      format!(
        "Cavell inspected {} for {} and found an empty root directory.",
        workspace_name, thread_title
      ),
      &relevant_memory_notes,
    );
  }

  let preview = entries
    .iter()
    .take(5)
    .map(|entry| entry.name.clone())
    .collect::<Vec<_>>()
    .join(", ");

  let fallback = format!(
    "Cavell inspected {} for {} and found {} root entries, including {}.",
    workspace_name,
    thread_title,
    entries.len(),
    preview
  );
  let prompt = format!(
    "You are Cavell, a concise local coding agent. Summarize a root directory inspection in one or two sentences.\nThread: {thread_title}\nWorkspace: {workspace_name}\n{}\nEntries:\n{}",
    format_memory_prompt(&relevant_memory_notes),
    format_directory_result(entries)
  );

  generate_local_summary(model_runtime, prompt, fallback, &relevant_memory_notes)
}

fn summarize_search_result(
  model_runtime: &LocalModelRuntime,
  memory_notes: &[MemoryNote],
  thread_title: &str,
  workspace_name: &str,
  query: &str,
  matches: &[SearchMatch],
) -> (String, HashMap<String, String>) {
  let relevant_memory_notes = retrieve_memory_context(memory_notes, Some(workspace_name), query);
  if matches.is_empty() {
    return generate_local_summary(
      model_runtime,
      format!(
        "You are Cavell, a concise local coding agent. Summarize a search with no matches.\nThread: {thread_title}\nWorkspace: {workspace_name}\n{}\nQuery: {query}",
        format_memory_prompt(&relevant_memory_notes)
      ),
      format!(
        "Cavell searched {} for {} and found no matches for \"{}\".",
        workspace_name, thread_title, query
      ),
      &relevant_memory_notes,
    );
  }

  let preview = matches
    .iter()
    .take(3)
    .map(|entry| format!("{}:{}", entry.relative_path, entry.line_number))
    .collect::<Vec<_>>()
    .join(", ");

  let fallback = format!(
    "Cavell searched {} for {} and found {} matches for \"{}\", including {}.",
    workspace_name,
    thread_title,
    matches.len(),
    query,
    preview
  );
  let prompt = format!(
    "You are Cavell, a concise local coding agent. Summarize a workspace search in one or two sentences.\nThread: {thread_title}\nWorkspace: {workspace_name}\n{}\nQuery: {query}\nMatches:\n{}",
    format_memory_prompt(&relevant_memory_notes),
    format_search_result(query, matches)
  );

  generate_local_summary(model_runtime, prompt, fallback, &relevant_memory_notes)
}

fn format_shell_result(result: &ShellCommandResult) -> String {
  let stdout = if result.stdout.trim().is_empty() {
    "[no stdout]".to_string()
  } else {
    result.stdout.clone()
  };
  let stderr = if result.stderr.trim().is_empty() {
    "[no stderr]".to_string()
  } else {
    result.stderr.clone()
  };
  let truncation_note = if result.was_truncated {
    "\n\n[output truncated]"
  } else {
    ""
  };

  format!(
    "Command: {}\nExit Code: {}\n\nstdout:\n{}\n\nstderr:\n{}{}",
    result.command, result.exit_code, stdout, stderr, truncation_note
  )
}

fn summarize_shell_result(
  model_runtime: &LocalModelRuntime,
  memory_notes: &[MemoryNote],
  workspace_name: &str,
  result: &ShellCommandResult,
) -> (String, HashMap<String, String>) {
  let relevant_memory_notes =
    retrieve_memory_context(memory_notes, Some(workspace_name), &result.command);
  let fallback = if result.exit_code == 0 {
    format!(
      "Cavell ran `{}` in {} and it finished successfully.",
      result.command, workspace_name
    )
  } else {
    format!(
      "Cavell ran `{}` in {} and it exited with code {}.",
      result.command, workspace_name, result.exit_code
    )
  };
  let prompt = format!(
    "You are Cavell, a concise local coding agent. Summarize a shell command result in one or two sentences.\nWorkspace: {workspace_name}\n{}\nCommand: {}\nExit Code: {}\nstdout:\n{}\n\nstderr:\n{}",
    format_memory_prompt(&relevant_memory_notes),
    result.command,
    result.exit_code,
    result.stdout,
    result.stderr
  );

  generate_local_summary(model_runtime, prompt, fallback, &relevant_memory_notes)
}

fn summarize_denied_approval(
  model_runtime: &LocalModelRuntime,
  memory_notes: &[MemoryNote],
  workspace_name: &str,
  approval: &PendingApproval,
) -> (String, HashMap<String, String>) {
  let query = approval
    .command
    .clone()
    .unwrap_or_else(|| format!("{} {}", approval.action, approval.relative_path));
  let relevant_memory_notes = retrieve_memory_context(memory_notes, Some(workspace_name), &query);
  let fallback = if approval.action == "run_shell" {
    let command = approval.command.clone().unwrap_or_default();
    format!(
      "Cavell skipped the shell command `{}` because the approval was denied.",
      command
    )
  } else {
    format!(
      "Cavell skipped writing {} because the approval was denied.",
      approval.relative_path
    )
  };
  let prompt = format!(
    "You are Cavell, a concise local coding agent. Summarize a denied approval in one sentence.\nWorkspace: {workspace_name}\n{}\nAction: {}\nTarget: {}\nCommand: {}",
    format_memory_prompt(&relevant_memory_notes),
    approval.action,
    approval.relative_path,
    approval.command.clone().unwrap_or_default()
  );

  generate_local_summary(model_runtime, prompt, fallback, &relevant_memory_notes)
}

fn generate_local_summary(
  model_runtime: &LocalModelRuntime,
  prompt: String,
  fallback: String,
  memory_notes: &[MemoryNote],
) -> (String, HashMap<String, String>) {
  let result = model_runtime.generate(GenerateRequest {
    role: ModelRole::Summarizer,
    prompt,
    fallback,
    max_tokens: 160,
  });

  let mut attributes = HashMap::from([
    ("modelId".to_string(), result.model_id),
    ("modelBackend".to_string(), result.backend),
    ("modelStatus".to_string(), result.status),
  ]);
  merge_memory_attributes(&mut attributes, memory_notes);

  (result.text, attributes)
}

fn retrieve_memory_context(
  memory_notes: &[MemoryNote],
  workspace_scope: Option<&str>,
  query: &str,
) -> Vec<MemoryNote> {
  retrieve_relevant_notes(memory_notes, workspace_scope, query, 3)
}

fn format_memory_prompt(memory_notes: &[MemoryNote]) -> String {
  if memory_notes.is_empty() {
    return "Memory: none.".to_string();
  }

  let note_lines = memory_notes
    .iter()
    .map(|note| {
      format!(
        "- {} | scope={} | source={} | {}",
        note.title, note.scope, note.source, note.body
      )
    })
    .collect::<Vec<_>>()
    .join("\n");

  format!("Relevant memory notes:\n{note_lines}")
}

fn merge_memory_attributes(attributes: &mut HashMap<String, String>, memory_notes: &[MemoryNote]) {
  attributes.insert(
    "memoryNoteCount".to_string(),
    memory_notes.len().to_string(),
  );
  if memory_notes.is_empty() {
    return;
  }

  attributes.insert(
    "memoryNoteIds".to_string(),
    memory_notes
      .iter()
      .map(|note| note.id.clone())
      .collect::<Vec<_>>()
      .join(", "),
  );
  attributes.insert(
    "memoryNoteTitles".to_string(),
    memory_notes
      .iter()
      .map(|note| note.title.clone())
      .collect::<Vec<_>>()
      .join(" | "),
  );
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
mod tests {
  use super::*;
  use serde_json::{json, Value};
  use std::env;
  use std::thread;
  use std::time::{SystemTime, UNIX_EPOCH};

  fn request(method: &str, params: Option<Value>) -> JsonRpcRequest {
    JsonRpcRequest {
      id: json!(1),
      method: method.to_string(),
      params,
    }
  }

  fn create_temp_workspace(label: &str) -> PathBuf {
    let unique = SystemTime::now()
      .duration_since(UNIX_EPOCH)
      .expect("system time")
      .as_nanos();
    let path = env::temp_dir().join(format!("cavell-{label}-{unique}"));
    fs::create_dir_all(&path).expect("create temp workspace");
    path
  }

  #[test]
  fn initialize_request_returns_capabilities() {
    let mut context = RuntimeContext::new_in_memory();
    let response = handle_request(
      &mut context,
      request(
        methods::INITIALIZE,
        Some(json!({
          "clientInfo": {
            "name": "cavell-tests",
            "version": "0.1.0"
          }
        })),
      ),
    );

    assert!(response.error.is_none());
    let result = response.result.expect("initialize result");
    assert_eq!(result["protocolVersion"], "0.1.0");
    assert_eq!(result["capabilities"]["supportsThreads"], true);
    assert_eq!(result["capabilities"]["supportsTools"], true);
  }

  #[test]
  fn health_ping_returns_ok() {
    let mut context = RuntimeContext::new_in_memory();
    let response = handle_request(&mut context, request(methods::HEALTH_PING, None));

    assert!(response.error.is_none());
    let result = response.result.expect("health result");
    assert_eq!(result["status"], "ok");
  }

  #[test]
  fn model_health_returns_local_model_status() {
    let mut context = RuntimeContext::new_in_memory();
    let response = handle_request(&mut context, request(methods::MODEL_HEALTH, None));

    assert!(response.error.is_none());
    let result = response.result.expect("model health result");
    assert_eq!(result["displayName"], "LFM2.5-350M");
    assert!(result["backend"].is_string());
    assert!(result["status"].is_string());
  }

  #[test]
  fn unknown_method_returns_json_rpc_error() {
    let mut context = RuntimeContext::new_in_memory();
    let response = handle_request(&mut context, request("unknown/method", None));

    assert!(response.result.is_none());
    let error = response.error.expect("error payload");
    assert_eq!(error.code, -32601);
  }

  #[test]
  fn workspace_open_sets_runtime_workspace() {
    let mut context = RuntimeContext::new_in_memory();
    let workspace = create_temp_workspace("open");

    let response = handle_request(
      &mut context,
      request(
        methods::WORKSPACE_OPEN,
        Some(json!({
          "path": workspace.display().to_string()
        })),
      ),
    );

    fs::remove_dir_all(&workspace).expect("cleanup temp workspace");

    assert!(response.error.is_none());
    let result = response.result.expect("workspace open result");
    assert_eq!(
      result["workspace"]["displayName"].as_str().unwrap(),
      workspace.file_name().unwrap().to_string_lossy()
    );
  }

  #[test]
  fn thread_start_persists_thread_for_future_lists() {
    let mut context = RuntimeContext::new_in_memory();

    let start_response = handle_request(
      &mut context,
      request(
        methods::THREAD_START,
        Some(json!({
          "title": "First Thread"
        })),
      ),
    );
    assert!(start_response.error.is_none());

    let list_response = handle_request(&mut context, request(methods::THREAD_LIST, None));
    let result = list_response.result.expect("thread list result");
    let threads = result["threads"].as_array().expect("thread array");

    assert_eq!(threads.len(), 1);
    assert_eq!(threads[0]["title"], "First Thread");
  }

  #[test]
  fn thread_read_returns_persisted_thread_items() {
    let mut context = RuntimeContext::new_in_memory();

    let _ = handle_request(
      &mut context,
      request(
        methods::THREAD_START,
        Some(json!({
          "title": "Inspectable Thread"
        })),
      ),
    );

    let read_response = handle_request(
      &mut context,
      request(
        methods::THREAD_READ,
        Some(json!({
          "threadId": "thread-1"
        })),
      ),
    );

    assert!(read_response.error.is_none());
    let result = read_response.result.expect("thread read result");
    let items = result["items"].as_array().expect("thread items");

    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["kind"], "system");
  }

  #[test]
  fn turn_start_warns_when_workspace_is_missing() {
    let mut context = RuntimeContext::new_in_memory();

    let _ = handle_request(
      &mut context,
      request(
        methods::THREAD_START,
        Some(json!({
          "title": "Chat Thread"
        })),
      ),
    );

    let turn_response = handle_request(
      &mut context,
      request(
        methods::TURN_START,
        Some(json!({
          "threadId": "thread-1",
          "message": "Inspect the project"
        })),
      ),
    );

    assert!(turn_response.error.is_none());
    let result = turn_response.result.expect("turn result");
    let items = result["items"].as_array().expect("items");

    assert_eq!(items[0]["kind"], "userMessage");
    assert_eq!(items[1]["kind"], "plan");
    assert_eq!(items[2]["kind"], "warning");
  }

  #[test]
  fn turn_start_reads_a_requested_workspace_file() {
    let mut context = RuntimeContext::new_in_memory();
    let workspace = create_temp_workspace("read-file");
    fs::write(
      workspace.join("README.md"),
      "# Milestone 1\nWorkspace tool test\n",
    )
    .expect("write readme");

    let _ = handle_request(
      &mut context,
      request(
        methods::WORKSPACE_OPEN,
        Some(json!({
          "path": workspace.display().to_string()
        })),
      ),
    );
    let _ = handle_request(
      &mut context,
      request(
        methods::THREAD_START,
        Some(json!({
          "title": "Workspace Thread"
        })),
      ),
    );

    let turn_response = handle_request(
      &mut context,
      request(
        methods::TURN_START,
        Some(json!({
          "threadId": "thread-1",
          "message": "Read README.md"
        })),
      ),
    );

    fs::remove_dir_all(&workspace).expect("cleanup temp workspace");

    assert!(turn_response.error.is_none());
    let result = turn_response.result.expect("turn result");
    let items = result["items"].as_array().expect("items");

    assert_eq!(items[1]["kind"], "plan");
    assert_eq!(items[1]["attributes"]["responseRole"], "planner");
    assert_eq!(items[1]["attributes"]["memoryNoteCount"], "1");
    assert_eq!(items[1]["attributes"]["memoryNoteIds"], "memory-1");
    assert_eq!(items[2]["kind"], "toolStart");
    assert_eq!(items[3]["kind"], "toolResult");
    assert_eq!(items[4]["kind"], "assistantMessage");
    assert_eq!(items[4]["attributes"]["responseRole"], "summarizer");
    assert_eq!(items[4]["attributes"]["memoryNoteCount"], "1");
    assert!(items[4]["attributes"]["memoryNoteTitles"]
      .as_str()
      .unwrap()
      .contains("Opened workspace"));
    assert!(matches!(
      items[4]["attributes"]["streamingStatus"].as_str(),
      Some("in_progress") | Some("completed")
    ));
    assert!(items[3]["content"]
      .as_str()
      .unwrap()
      .contains("Milestone 1"));
    assert_eq!(result["activeTurnId"].as_str().unwrap(), "thread-1-turn-1");
  }

  #[test]
  fn collect_notifications_emits_thread_update_for_active_turn() {
    let mut context = RuntimeContext::new_in_memory();
    let workspace = create_temp_workspace("thread-updated");
    fs::write(
      workspace.join("README.md"),
      "# Cavell\nNotification coverage\n",
    )
    .expect("write readme");

    let _ = handle_request(
      &mut context,
      request(
        methods::WORKSPACE_OPEN,
        Some(json!({
          "path": workspace.display().to_string()
        })),
      ),
    );
    let _ = handle_request(
      &mut context,
      request(
        methods::THREAD_START,
        Some(json!({
          "title": "Notification Thread"
        })),
      ),
    );
    let turn_response = handle_request(
      &mut context,
      request(
        methods::TURN_START,
        Some(json!({
          "threadId": "thread-1",
          "message": "Read README.md"
        })),
      ),
    );

    assert!(turn_response.error.is_none());
    thread::sleep(std::time::Duration::from_millis(260));

    let notifications = collect_notifications(&mut context).expect("collect notifications");

    fs::remove_dir_all(&workspace).expect("cleanup temp workspace");

    assert_eq!(notifications.len(), 1);
    assert_eq!(
      notifications[0].method,
      methods::THREAD_UPDATED_NOTIFICATION
    );
    let params = notifications[0]
      .params
      .as_ref()
      .expect("notification params");
    let items = params["items"].as_array().expect("notification items");
    let assistant_item = items
      .iter()
      .rev()
      .find(|item| item["kind"] == "assistantMessage")
      .expect("assistant message");
    assert!(
      assistant_item["attributes"]["streamedCharacters"]
        .as_str()
        .expect("streamed chars")
        .parse::<usize>()
        .expect("streamed chars usize")
        > 0
    );
  }

  #[test]
  fn turn_cancel_stops_an_active_assistant_response() {
    let mut context = RuntimeContext::new_in_memory();
    let workspace = create_temp_workspace("turn-cancel");
    fs::write(
      workspace.join("README.md"),
      "# Milestone 1\nStreaming turn content\n",
    )
    .expect("write readme");

    let _ = handle_request(
      &mut context,
      request(
        methods::WORKSPACE_OPEN,
        Some(json!({
          "path": workspace.display().to_string()
        })),
      ),
    );
    let _ = handle_request(
      &mut context,
      request(
        methods::THREAD_START,
        Some(json!({
          "title": "Streaming Thread"
        })),
      ),
    );

    let turn_response = handle_request(
      &mut context,
      request(
        methods::TURN_START,
        Some(json!({
          "threadId": "thread-1",
          "message": "Read README.md"
        })),
      ),
    );
    let turn_result = turn_response.result.expect("turn result");
    assert_eq!(turn_result["activeTurnId"], "thread-1-turn-1");

    let cancel_response = handle_request(
      &mut context,
      request(
        methods::TURN_CANCEL,
        Some(json!({
          "turnId": "thread-1-turn-1"
        })),
      ),
    );

    fs::remove_dir_all(&workspace).expect("cleanup temp workspace");

    assert!(cancel_response.error.is_none());
    let cancel_result = cancel_response.result.expect("cancel result");
    let items = cancel_result["items"].as_array().expect("cancel items");

    assert_eq!(items[0]["title"], "Turn Cancelled");
    assert_eq!(cancel_result["activeTurnId"], serde_json::Value::Null);
  }

  #[test]
  fn turn_start_searches_workspace_content() {
    let mut context = RuntimeContext::new_in_memory();
    let workspace = create_temp_workspace("search-files");
    fs::write(
      workspace.join("README.md"),
      "# Cavell\nSearch target lives here\n",
    )
    .expect("write readme");
    fs::create_dir_all(workspace.join("docs")).expect("create docs directory");
    fs::write(
      workspace.join("docs").join("notes.txt"),
      "Another Search target appears in docs\n",
    )
    .expect("write notes");

    let _ = handle_request(
      &mut context,
      request(
        methods::WORKSPACE_OPEN,
        Some(json!({
          "path": workspace.display().to_string()
        })),
      ),
    );
    let _ = handle_request(
      &mut context,
      request(
        methods::THREAD_START,
        Some(json!({
          "title": "Search Thread"
        })),
      ),
    );

    let turn_response = handle_request(
      &mut context,
      request(
        methods::TURN_START,
        Some(json!({
          "threadId": "thread-1",
          "message": "Find Search target"
        })),
      ),
    );

    fs::remove_dir_all(&workspace).expect("cleanup temp workspace");

    assert!(turn_response.error.is_none());
    let result = turn_response.result.expect("turn result");
    let items = result["items"].as_array().expect("items");

    assert_eq!(items[2]["kind"], "toolStart");
    assert_eq!(items[2]["title"], "search_files");
    assert_eq!(items[3]["kind"], "toolResult");
    assert!(items[3]["content"]
      .as_str()
      .unwrap()
      .contains("README.md:2"));
    assert!(items[3]["content"]
      .as_str()
      .unwrap()
      .contains("docs/notes.txt:1"));
  }

  #[test]
  fn approval_respond_writes_file_after_approval() {
    let mut context = RuntimeContext::new_in_memory();
    let workspace = create_temp_workspace("approval-write");

    let _ = handle_request(
      &mut context,
      request(
        methods::WORKSPACE_OPEN,
        Some(json!({
          "path": workspace.display().to_string()
        })),
      ),
    );
    let _ = handle_request(
      &mut context,
      request(
        methods::THREAD_START,
        Some(json!({
          "title": "Approval Thread"
        })),
      ),
    );

    let turn_response = handle_request(
      &mut context,
      request(
        methods::TURN_START,
        Some(json!({
          "threadId": "thread-1",
          "message": "Write docs/output.txt: Approval protected content"
        })),
      ),
    );

    assert!(turn_response.error.is_none());
    let turn_result = turn_response.result.expect("turn result");
    let turn_items = turn_result["items"].as_array().expect("turn items");
    assert_eq!(turn_items[2]["title"], "generate_diff");
    assert_eq!(turn_items[3]["kind"], "diffArtifact");
    assert!(turn_items[3]["content"]
      .as_str()
      .unwrap()
      .contains("+++ b/docs/output.txt"));
    assert_eq!(turn_items[4]["kind"], "approvalRequested");
    let approval_id = turn_result["pendingApprovals"][0]["id"]
      .as_str()
      .expect("approval id")
      .to_string();

    let approval_response = handle_request(
      &mut context,
      request(
        methods::APPROVAL_RESPOND,
        Some(json!({
          "approvalId": approval_id,
          "decision": "approved"
        })),
      ),
    );

    let written_content =
      fs::read_to_string(workspace.join("docs").join("output.txt")).expect("read written output");
    fs::remove_dir_all(&workspace).expect("cleanup temp workspace");

    assert!(approval_response.error.is_none());
    let approval_result = approval_response.result.expect("approval result");
    let items = approval_result["items"].as_array().expect("approval items");

    assert_eq!(items[0]["kind"], "approvalResolved");
    assert_eq!(items[1]["title"], "write_file");
    assert_eq!(written_content, "Approval protected content");
  }

  #[test]
  fn approval_respond_runs_shell_after_approval() {
    let mut context = RuntimeContext::new_in_memory();
    let workspace = create_temp_workspace("approval-shell");
    fs::write(workspace.join("marker.txt"), "shell target\n").expect("write shell marker");

    let _ = handle_request(
      &mut context,
      request(
        methods::WORKSPACE_OPEN,
        Some(json!({
          "path": workspace.display().to_string()
        })),
      ),
    );
    let _ = handle_request(
      &mut context,
      request(
        methods::THREAD_START,
        Some(json!({
          "title": "Shell Thread"
        })),
      ),
    );

    let turn_response = handle_request(
      &mut context,
      request(
        methods::TURN_START,
        Some(json!({
          "threadId": "thread-1",
          "message": "Run shell: ls"
        })),
      ),
    );

    assert!(turn_response.error.is_none());
    let turn_result = turn_response.result.expect("turn result");
    let approval_id = turn_result["pendingApprovals"][0]["id"]
      .as_str()
      .expect("approval id")
      .to_string();

    let approval_response = handle_request(
      &mut context,
      request(
        methods::APPROVAL_RESPOND,
        Some(json!({
          "approvalId": approval_id,
          "decision": "approved"
        })),
      ),
    );

    fs::remove_dir_all(&workspace).expect("cleanup temp workspace");

    assert!(approval_response.error.is_none());
    let approval_result = approval_response.result.expect("approval result");
    let items = approval_result["items"].as_array().expect("approval items");

    assert_eq!(items[1]["title"], "run_shell");
    assert!(items[2]["content"].as_str().unwrap().contains("marker.txt"));
  }

  #[test]
  fn follow_up_turn_retrieves_recent_memory_notes() {
    let mut context = RuntimeContext::new_in_memory();
    let workspace = create_temp_workspace("memory-follow-up");

    let _ = handle_request(
      &mut context,
      request(
        methods::WORKSPACE_OPEN,
        Some(json!({
          "path": workspace.display().to_string()
        })),
      ),
    );
    let _ = handle_request(
      &mut context,
      request(
        methods::THREAD_START,
        Some(json!({
          "title": "Memory Thread"
        })),
      ),
    );

    let write_turn = handle_request(
      &mut context,
      request(
        methods::TURN_START,
        Some(json!({
          "threadId": "thread-1",
          "message": "Write docs/output.txt: Memory connected content"
        })),
      ),
    );
    let approval_id = write_turn.result.expect("write turn result")["pendingApprovals"][0]["id"]
      .as_str()
      .expect("approval id")
      .to_string();

    let approval_response = handle_request(
      &mut context,
      request(
        methods::APPROVAL_RESPOND,
        Some(json!({
          "approvalId": approval_id,
          "decision": "approved"
        })),
      ),
    );
    assert!(approval_response.error.is_none());

    let follow_up_turn = handle_request(
      &mut context,
      request(
        methods::TURN_START,
        Some(json!({
          "threadId": "thread-1",
          "message": "Read docs/output.txt"
        })),
      ),
    );

    fs::remove_dir_all(&workspace).expect("cleanup temp workspace");

    assert!(follow_up_turn.error.is_none());
    let items = follow_up_turn.result.expect("follow-up turn result")["items"]
      .as_array()
      .expect("follow-up items")
      .clone();

    assert_eq!(items[1]["attributes"]["memoryNoteCount"], "2");
    assert!(items[1]["attributes"]["memoryNoteTitles"]
      .as_str()
      .unwrap()
      .contains("Wrote docs/output.txt"));
    assert!(items[4]["attributes"]["memoryNoteTitles"]
      .as_str()
      .unwrap()
      .contains("Wrote docs/output.txt"));
  }
}
