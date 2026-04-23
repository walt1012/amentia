use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;
use cavell_protocol::{
  methods, HealthPingResult, InitializeParams, InitializeResult, JsonRpcRequest, JsonRpcResponse,
  ServerCapabilities, ServerInfo, ThreadListResult, ThreadReadParams, ThreadReadResult,
  ThreadStartParams, ThreadStartResult, ThreadSummary, TimelineItem, TurnStartParams,
  TurnStartResult, WorkspaceOpenParams, WorkspaceOpenResult, WorkspaceSummary,
};
use cavell_storage::{FileThreadStore, StoredThreadRecord};
use cavell_tools::{list_directory, read_file, DirectoryEntry, ReadFileResult};

#[derive(Debug, Clone)]
struct StoredThread {
  summary: ThreadSummary,
  turn_count: usize,
  items: Vec<TimelineItem>,
}

#[derive(Debug, Clone)]
pub struct RuntimeContext {
  server_name: String,
  server_version: String,
  store: Option<FileThreadStore>,
  threads: Vec<StoredThread>,
  workspace: Option<WorkspaceSummary>,
  next_thread_number: usize,
}

impl RuntimeContext {
  pub fn new() -> Result<Self> {
    let store = FileThreadStore::new_default()?;
    let persisted_threads = store.load_threads()?;
    let next_thread_number = persisted_threads.len() + 1;

    Ok(Self {
      server_name: "cavell-runtime".to_string(),
      server_version: env!("CARGO_PKG_VERSION").to_string(),
      store: Some(store),
      threads: persisted_threads
        .into_iter()
        .map(|thread| StoredThread {
          summary: thread.summary,
          turn_count: thread.turn_count,
          items: thread.items,
        })
        .collect(),
      workspace: None,
      next_thread_number,
    })
  }

  pub fn new_in_memory() -> Self {
    Self {
      server_name: "cavell-runtime".to_string(),
      server_version: env!("CARGO_PKG_VERSION").to_string(),
      store: None,
      threads: vec![],
      workspace: None,
      next_thread_number: 1,
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
}

impl Default for RuntimeContext {
  fn default() -> Self {
    Self::new_in_memory()
  }
}

pub fn handle_request(context: &mut RuntimeContext, request: JsonRpcRequest) -> JsonRpcResponse {
  match request.method.as_str() {
    methods::INITIALIZE => handle_initialize(context, request),
    methods::HEALTH_PING => JsonRpcResponse::success(
      request.id,
      &HealthPingResult {
        status: "ok".to_string(),
      },
    ),
    methods::WORKSPACE_OPEN => handle_workspace_open(context, request),
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
        supports_threads: true,
        supports_tools: true,
        supports_plugins: false,
      },
    },
  )
}

fn handle_workspace_open(
  context: &mut RuntimeContext,
  request: JsonRpcRequest,
) -> JsonRpcResponse {
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

  JsonRpcResponse::success(
    request.id,
    &WorkspaceOpenResult {
      workspace,
      thread_count: context.threads.len(),
    },
  )
}

fn handle_thread_read(context: &RuntimeContext, request: JsonRpcRequest) -> JsonRpcResponse {
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
  }];
  context.threads.push(StoredThread {
    summary: thread.clone(),
    turn_count: 0,
    items: items.clone(),
  });

  if let Err(error) = context.persist_threads() {
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

  thread.summary.status = match &workspace {
    Some(workspace) => format!("{turn_count} turn(s) in {}", workspace.display_name),
    None => format!("{turn_count} turn(s)"),
  };

  let mut items = vec![TimelineItem {
    kind: "userMessage".to_string(),
    title: "User".to_string(),
    content: message.clone(),
  }];

  if let Some(workspace) = workspace {
    let workspace_root = Path::new(&workspace.root_path);

    if let Some(relative_path) = infer_requested_file_path(&message, workspace_root) {
      items.push(TimelineItem {
        kind: "plan".to_string(),
        title: "Plan".to_string(),
        content: format!(
          "Inspect {} in {} with the built-in read_file tool.",
          relative_path, workspace.display_name
        ),
      });
      items.push(TimelineItem {
        kind: "toolStart".to_string(),
        title: "read_file".to_string(),
        content: relative_path.clone(),
      });

      match read_file(workspace_root, &relative_path, 4096) {
        Ok(result) => {
          items.push(TimelineItem {
            kind: "toolResult".to_string(),
            title: "read_file result".to_string(),
            content: format_file_result(&result),
          });
          items.push(TimelineItem {
            kind: "assistantMessage".to_string(),
            title: "Assistant".to_string(),
            content: summarize_file_result(&thread_title, &workspace.display_name, &result),
          });
        }
        Err(error) => {
          items.push(TimelineItem {
            kind: "warning".to_string(),
            title: "read_file failed".to_string(),
            content: error.to_string(),
          });
          items.push(TimelineItem {
            kind: "assistantMessage".to_string(),
            title: "Assistant".to_string(),
            content: format!(
              "Cavell could not inspect that file in {}. Try another path inside the workspace.",
              workspace.display_name
            ),
          });
        }
      }
    } else {
      items.push(TimelineItem {
        kind: "plan".to_string(),
        title: "Plan".to_string(),
        content: format!(
          "Inspect the root of {} with the built-in list_directory tool.",
          workspace.display_name
        ),
      });
      items.push(TimelineItem {
        kind: "toolStart".to_string(),
        title: "list_directory".to_string(),
        content: ".".to_string(),
      });

      match list_directory(workspace_root, None, 24) {
        Ok(entries) => {
          items.push(TimelineItem {
            kind: "toolResult".to_string(),
            title: "list_directory result".to_string(),
            content: format_directory_result(&entries),
          });
          items.push(TimelineItem {
            kind: "assistantMessage".to_string(),
            title: "Assistant".to_string(),
            content: summarize_directory_result(&thread_title, &workspace.display_name, &entries),
          });
        }
        Err(error) => {
          items.push(TimelineItem {
            kind: "warning".to_string(),
            title: "list_directory failed".to_string(),
            content: error.to_string(),
          });
          items.push(TimelineItem {
            kind: "assistantMessage".to_string(),
            title: "Assistant".to_string(),
            content: format!(
              "Cavell could not inspect the root of {} yet. Re-open the workspace and try again.",
              workspace.display_name
            ),
          });
        }
      }
    }
  } else {
    items.push(TimelineItem {
      kind: "plan".to_string(),
      title: "Plan".to_string(),
      content: "Wait for a workspace before running filesystem tools.".to_string(),
    });
    items.push(TimelineItem {
      kind: "warning".to_string(),
      title: "Workspace Required".to_string(),
      content: "Open a workspace before asking Cavell to inspect files.".to_string(),
    });
    items.push(TimelineItem {
      kind: "assistantMessage".to_string(),
      title: "Assistant".to_string(),
      content: format!(
        "Cavell received your message in {}, but Milestone 1 tools need an opened workspace first.",
        thread_title
      ),
    });
  }

  thread.items.extend(items.clone());

  if let Err(error) = context.persist_threads() {
    return JsonRpcResponse::error(request.id, -32010, error.to_string());
  }

  JsonRpcResponse::success(
    request.id,
    &TurnStartResult {
      turn_id: format!("{thread_id}-turn-{turn_count}"),
      thread_id,
      items,
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
  thread_title: &str,
  workspace_name: &str,
  result: &ReadFileResult,
) -> String {
  let preview = result
    .content
    .lines()
    .find(|line| !line.trim().is_empty())
    .unwrap_or("The file is empty.");

  format!(
    "Cavell inspected {} for {} in {}. First useful line: {}",
    result.relative_path, thread_title, workspace_name, preview
  )
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

fn summarize_directory_result(
  thread_title: &str,
  workspace_name: &str,
  entries: &[DirectoryEntry],
) -> String {
  if entries.is_empty() {
    return format!(
      "Cavell inspected {} for {} and found an empty root directory.",
      workspace_name, thread_title
    );
  }

  let preview = entries
    .iter()
    .take(5)
    .map(|entry| entry.name.clone())
    .collect::<Vec<_>>()
    .join(", ");

  format!(
    "Cavell inspected {} for {} and found {} root entries, including {}.",
    workspace_name,
    thread_title,
    entries.len(),
    preview
  )
}

#[cfg(test)]
mod tests {
  use super::*;
  use serde_json::{json, Value};
  use std::env;
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
    fs::write(workspace.join("README.md"), "# Milestone 1\nWorkspace tool test\n")
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

    assert_eq!(items[2]["kind"], "toolStart");
    assert_eq!(items[3]["kind"], "toolResult");
    assert!(items[3]["content"].as_str().unwrap().contains("Milestone 1"));
  }
}
