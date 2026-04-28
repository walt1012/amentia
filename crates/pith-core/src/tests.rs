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
  let path = env::temp_dir().join(format!("pith-{label}-{unique}"));
  fs::create_dir_all(&path).expect("create temp workspace");
  path
}

fn create_temp_plugin_bundle(label: &str, plugin_name: &str, display_name: &str) -> PathBuf {
  let root = create_temp_workspace(label);
  let plugin_dir = root.join(plugin_name);
  fs::create_dir_all(plugin_dir.join("commands")).expect("create plugin commands directory");
  fs::write(
    plugin_dir.join("pith-plugin.json"),
    format!(
      r#"{{
"name": "{plugin_name}",
"version": "0.1.0",
"displayName": "{display_name}",
"description": "Temporary test plugin",
"author": {{ "name": "Pith" }},
"capabilities": ["command:{plugin_name}.run"],
"permissions": ["file.read"],
"defaultEnabled": true
}}"#
    ),
  )
  .expect("write plugin manifest");
  fs::write(
    plugin_dir
      .join("commands")
      .join(format!("{plugin_name}.run.json")),
    r#"{
"title": "Run Temporary Plugin",
"description": "Execute a temporary plugin command.",
"prompt": "Summarize the local workspace in one paragraph."
}"#,
  )
  .expect("write command manifest");
  plugin_dir
}

fn enable_full_access_plugin(context: &mut RuntimeContext) {
  context.plugins = vec![PluginCatalogEntry {
    id: "test-full-access".to_string(),
    name: "test-full-access".to_string(),
    version: "0.1.0".to_string(),
    display_name: "Test Full Access".to_string(),
    status: "ready".to_string(),
    description: "Grants built-in workspace and shell permissions for tests".to_string(),
    author_name: Some("Pith".to_string()),
    enabled: true,
    default_enabled: true,
    capabilities: vec!["prompt_pack:test.full_access".to_string()],
    permissions: vec![
      "file.read".to_string(),
      "file.write".to_string(),
      "shell.exec".to_string(),
    ],
    manifest_path: "tests/test-full-access/pith-plugin.json".to_string(),
    provenance: "test".to_string(),
    validation_error: None,
    validation_hint: None,
  }];
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
          "name": "pith-tests",
          "version": "0.1.0"
        }
      })),
    ),
  );

  assert!(response.error.is_none());
  let result = response.result.expect("initialize result");
  assert_eq!(result["protocolVersion"], "0.1.0");
  assert_eq!(result["capabilities"]["supportsRuntimeReadiness"], true);
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
fn runtime_readiness_reports_agent_control_surface() {
  let mut context = RuntimeContext::new_in_memory();
  let response = handle_request(&mut context, request(methods::RUNTIME_READINESS, None));

  assert!(response.error.is_none());
  let result = response.result.expect("runtime readiness result");
  assert_eq!(result["status"], "setup_required");
  assert!(result["summary"]
    .as_str()
    .expect("summary")
    .contains("local agent work"));
  let check_ids = result["checks"]
    .as_array()
    .expect("checks")
    .iter()
    .filter_map(|check| check["id"].as_str())
    .collect::<Vec<_>>();
  assert!(check_ids.contains(&"localModel"));
  assert!(check_ids.contains(&"workspace"));
  assert!(check_ids.contains(&"thread"));
  assert!(check_ids.contains(&"firstRequest"));
  assert!(check_ids.contains(&"boundedRuntime"));
  assert_eq!(result["metrics"]["contextWindowTokens"], "4096");
  assert_eq!(result["metrics"]["workspaceThreadCount"], "0");
  assert_eq!(result["metrics"]["firstRequestSent"], "false");
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
fn turn_start_requires_ready_model_when_runtime_enforces_readiness() {
  let mut context = RuntimeContext::new_in_memory();
  context.enforce_model_readiness = true;
  let response = handle_request(
    &mut context,
    request(
      methods::TURN_START,
      Some(json!({
        "threadId": "local-welcome",
        "message": "Inspect the workspace"
      })),
    ),
  );

  let error = response.error.expect("model readiness error");
  assert_eq!(error.code, -32060);
  assert!(error.message.contains("Local model is not ready"));
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
fn runtime_readiness_tracks_workspace_thread_setup() {
  let mut context = RuntimeContext::new_in_memory();
  let workspace = create_temp_workspace("thread-readiness");

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
        "title": "Local Thread"
      })),
    ),
  );
  let response = handle_request(&mut context, request(methods::RUNTIME_READINESS, None));

  fs::remove_dir_all(&workspace).expect("cleanup temp workspace");

  assert!(response.error.is_none());
  let result = response.result.expect("runtime readiness result");
  let checks = result["checks"].as_array().expect("checks");
  let thread_check = checks
    .iter()
    .find(|check| check["id"] == "thread")
    .expect("thread check");
  let first_request_check = checks
    .iter()
    .find(|check| check["id"] == "firstRequest")
    .expect("first request check");
  assert_eq!(thread_check["status"], "ready");
  assert_eq!(first_request_check["status"], "ready_to_send");
  assert_eq!(result["metrics"]["workspaceThreadCount"], "1");
  assert_eq!(result["metrics"]["firstRequestSent"], "false");
}

#[test]
fn workspace_search_returns_matching_lines() {
  let mut context = RuntimeContext::new_in_memory();
  let workspace = create_temp_workspace("workspace-search");
  fs::write(
    workspace.join("README.md"),
    "Pith local search\nNothing else\n",
  )
  .expect("write searchable file");

  let _ = handle_request(
    &mut context,
    request(
      methods::WORKSPACE_OPEN,
      Some(json!({
        "path": workspace.display().to_string()
      })),
    ),
  );

  let response = handle_request(
    &mut context,
    request(
      methods::WORKSPACE_SEARCH,
      Some(json!({
        "query": "local",
        "maxResults": 8
      })),
    ),
  );

  fs::remove_dir_all(&workspace).expect("cleanup temp workspace");

  assert!(response.error.is_none());
  let result = response.result.expect("workspace search result");
  let matches = result["matches"].as_array().expect("matches");
  assert_eq!(matches.len(), 1);
  assert_eq!(matches[0]["relativePath"], "README.md");
  assert_eq!(matches[0]["lineNumber"], 1);
}

#[test]
fn memory_create_adds_manual_workspace_note() {
  let mut context = RuntimeContext::new_in_memory();
  let workspace = create_temp_workspace("memory-create");

  let _ = handle_request(
    &mut context,
    request(
      methods::WORKSPACE_OPEN,
      Some(json!({
        "path": workspace.display().to_string()
      })),
    ),
  );

  let create_response = handle_request(
    &mut context,
    request(
      methods::MEMORY_CREATE,
      Some(json!({
        "title": "Repository preference",
        "body": "Prefer small, reviewable patches.",
      })),
    ),
  );

  fs::remove_dir_all(&workspace).expect("cleanup temp workspace");

  assert!(create_response.error.is_none());
  let result = create_response.result.expect("memory create result");
  assert_eq!(result["note"]["title"], "Repository preference");
  assert_eq!(result["note"]["source"], "user");
  assert_eq!(context.memory_notes.len(), 2);
  assert_eq!(context.memory_notes[0].title, "Repository preference");
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
  enable_full_access_plugin(&mut context);
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
  assert_eq!(items[4]["attributes"]["observationTruncated"], "false");
  assert_eq!(items[4]["attributes"]["observationBudgetChars"], "1843");
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
  enable_full_access_plugin(&mut context);
  let workspace = create_temp_workspace("thread-updated");
  fs::write(
    workspace.join("README.md"),
    "# Pith\nNotification coverage\n",
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
  enable_full_access_plugin(&mut context);
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
  enable_full_access_plugin(&mut context);
  let workspace = create_temp_workspace("search-files");
  fs::write(
    workspace.join("README.md"),
    "# Pith\nSearch target lives here\n",
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
  enable_full_access_plugin(&mut context);
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
  context.plugins = vec![PluginCatalogEntry {
    id: "shell-recorder".to_string(),
    name: "shell-recorder".to_string(),
    version: "0.1.0".to_string(),
    display_name: "Shell Recorder".to_string(),
    status: "ready".to_string(),
    description: "Shell access plugin".to_string(),
    author_name: Some("Pith".to_string()),
    enabled: true,
    default_enabled: false,
    capabilities: vec![
      "hook:shell.recorder".to_string(),
      "tool:shell.timeline".to_string(),
    ],
    permissions: vec!["shell.exec".to_string()],
    manifest_path: PathBuf::from(env!("CARGO_MANIFEST_DIR"))
      .join("../../plugins/bundled/shell-recorder/pith-plugin.json")
      .display()
      .to_string(),
    provenance: "bundled".to_string(),
    validation_error: None,
    validation_hint: None,
  }];
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
  let turn_items = turn_result["items"].as_array().expect("turn items");
  assert_eq!(
    turn_items[2]["attributes"]["sandboxMode"],
    "workspaceReadWrite"
  );
  assert!(turn_items[2]["content"]
    .as_str()
    .expect("approval content")
    .contains("Sandbox:"));
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
  assert_eq!(items[2]["attributes"]["sandboxMode"], "workspaceReadWrite");
  assert!(items[2]["attributes"]["sandboxBackend"].is_string());
  assert!(items[2]["attributes"]["sandboxActive"].is_string());
  assert!(items.iter().any(|item| item["kind"] == "pluginHook"));
  assert!(items.iter().any(|item| {
    item["title"] == "Record Shell Completion"
      && item["attributes"]["hookEvent"] == "shell.completed"
      && item["attributes"]["sandboxMode"] == "workspaceReadWrite"
  }));
  assert!(items.iter().any(|item| {
    item["title"] == "Hook Memory Note Saved"
      && item["attributes"]["memoryNoteTitle"] == "Shell Completion"
  }));
  assert!(context
    .memory_notes
    .iter()
    .any(|note| note.title == "Shell Completion" && note.source == "plugin.shell-recorder"));
}

#[test]
fn thread_summary_memory_note_is_updated_after_approval_resolution() {
  let mut context = RuntimeContext::new_in_memory();
  enable_full_access_plugin(&mut context);
  let workspace = create_temp_workspace("thread-summary");

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
        "title": "Summary Thread"
      })),
    ),
  );

  let turn_response = handle_request(
    &mut context,
    request(
      methods::TURN_START,
      Some(json!({
        "threadId": "thread-1",
        "message": "Write docs/output.txt: Summary content"
      })),
    ),
  );
  let approval_id = turn_response.result.expect("turn result")["pendingApprovals"][0]["id"]
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
  let summary_note = context
    .memory_notes
    .iter()
    .find(|note| note.id == "memory-thread-summary-thread-1")
    .expect("thread summary note");
  assert_eq!(summary_note.source, "thread");
  assert_eq!(summary_note.title, "Thread summary: Summary Thread");
  assert!(summary_note.body.contains("docs/output.txt"));
}

#[test]
fn follow_up_turn_retrieves_recent_memory_notes() {
  let mut context = RuntimeContext::new_in_memory();
  enable_full_access_plugin(&mut context);
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

  assert_eq!(items[1]["attributes"]["memoryNoteCount"], "3");
  assert_eq!(items[1]["attributes"]["contextWindowTokens"], "4096");
  assert_eq!(items[1]["attributes"]["contextBudgetChars"], "1228");
  assert!(items[1]["attributes"]["memoryNoteTitles"]
    .as_str()
    .unwrap()
    .contains("Wrote docs/output.txt"));
  assert!(items[4]["attributes"]["memoryNoteTitles"]
    .as_str()
    .unwrap()
    .contains("Wrote docs/output.txt"));
  assert_eq!(items[4]["attributes"]["memoryNoteCount"], "3");
  assert_eq!(items[4]["attributes"]["contextWindowTokens"], "4096");
}

#[test]
fn thread_turns_stay_bound_to_the_thread_workspace() {
  let mut context = RuntimeContext::new_in_memory();
  enable_full_access_plugin(&mut context);
  let workspace_a = create_temp_workspace("workspace-a");
  let workspace_b = create_temp_workspace("workspace-b");
  fs::write(
    workspace_a.join("README.md"),
    "# Workspace A\nThread-bound content\n",
  )
  .expect("write workspace a readme");
  fs::write(
    workspace_b.join("README.md"),
    "# Workspace B\nDifferent content\n",
  )
  .expect("write workspace b readme");

  let _ = handle_request(
    &mut context,
    request(
      methods::WORKSPACE_OPEN,
      Some(json!({
        "path": workspace_a.display().to_string()
      })),
    ),
  );
  let _ = handle_request(
    &mut context,
    request(
      methods::THREAD_START,
      Some(json!({
        "title": "Workspace Bound Thread"
      })),
    ),
  );
  let _ = handle_request(
    &mut context,
    request(
      methods::WORKSPACE_OPEN,
      Some(json!({
        "path": workspace_b.display().to_string()
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

  fs::remove_dir_all(&workspace_a).expect("cleanup workspace a");
  fs::remove_dir_all(&workspace_b).expect("cleanup workspace b");

  assert!(turn_response.error.is_none());
  let result = turn_response.result.expect("turn result");
  let items = result["items"].as_array().expect("items");
  assert!(items[3]["content"]
    .as_str()
    .unwrap()
    .contains("Workspace A"));
  assert!(items[3]["content"]
    .as_str()
    .unwrap()
    .contains("Thread-bound content"));
}

#[test]
fn plugin_set_enabled_updates_runtime_catalog() {
  let mut context = RuntimeContext::new_in_memory();
  context.plugins = vec![PluginCatalogEntry {
    id: "workspace-notes".to_string(),
    name: "workspace-notes".to_string(),
    version: "0.1.0".to_string(),
    display_name: "Workspace Notes".to_string(),
    status: "ready".to_string(),
    description: "Test plugin".to_string(),
    author_name: Some("Pith".to_string()),
    enabled: false,
    default_enabled: false,
    capabilities: vec!["prompt_pack:workspace.notes".to_string()],
    permissions: vec!["file.read".to_string()],
    manifest_path: "plugins/bundled/workspace-notes/pith-plugin.json".to_string(),
    provenance: "bundled".to_string(),
    validation_error: None,
    validation_hint: None,
  }];

  let response = handle_request(
    &mut context,
    request(
      methods::PLUGIN_SET_ENABLED,
      Some(json!({
        "pluginId": "workspace-notes",
        "enabled": true
      })),
    ),
  );

  assert!(response.error.is_none());
  assert!(context.plugins[0].enabled);
  assert_eq!(
    response.result.expect("plugin set result")["plugin"]["enabled"],
    true
  );
}

#[test]
fn plugin_install_adds_local_plugin_to_the_runtime_catalog() {
  let mut context = RuntimeContext::new_in_memory();
  let source_root =
    create_temp_plugin_bundle("plugin-install-source", "focus-review", "Focus Review");
  let install_root = create_temp_workspace("plugin-install-root");
  context.plugin_roots = vec![install_root.clone()];
  context.plugin_install_root = install_root.clone();
  context.plugins = vec![];

  let response = handle_request(
    &mut context,
    request(
      methods::PLUGIN_INSTALL,
      Some(json!({
        "sourcePath": source_root.display().to_string()
      })),
    ),
  );

  fs::remove_dir_all(source_root.parent().expect("plugin source root"))
    .expect("cleanup plugin source root");
  fs::remove_dir_all(&install_root).expect("cleanup install root");

  assert!(response.error.is_none());
  let result = response.result.expect("plugin install result");
  assert_eq!(result["plugin"]["id"], "focus-review");
  assert_eq!(result["plugin"]["provenance"], "local");
  assert!(context
    .plugins
    .iter()
    .any(|plugin| plugin.id == "focus-review"));
}

#[test]
fn plugin_install_rejects_duplicate_plugin_ids() {
  let mut context = RuntimeContext::new_in_memory();
  let source_root = create_temp_plugin_bundle(
    "plugin-install-duplicate",
    "workspace-notes",
    "Workspace Notes",
  );
  context.plugins = vec![PluginCatalogEntry {
    id: "workspace-notes".to_string(),
    name: "workspace-notes".to_string(),
    version: "0.1.0".to_string(),
    display_name: "Workspace Notes".to_string(),
    status: "ready".to_string(),
    description: "bundled plugin".to_string(),
    author_name: Some("Pith".to_string()),
    enabled: true,
    default_enabled: true,
    capabilities: vec!["prompt_pack:workspace.notes".to_string()],
    permissions: vec!["file.read".to_string()],
    manifest_path: "plugins/bundled/workspace-notes/pith-plugin.json".to_string(),
    provenance: "bundled".to_string(),
    validation_error: None,
    validation_hint: None,
  }];

  let response = handle_request(
    &mut context,
    request(
      methods::PLUGIN_INSTALL,
      Some(json!({
        "sourcePath": source_root.display().to_string()
      })),
    ),
  );

  fs::remove_dir_all(source_root.parent().expect("plugin source root"))
    .expect("cleanup plugin source root");

  assert!(response.result.is_none());
  let error = response.error.expect("plugin install error");
  assert!(error.message.contains("already installed"));
}

#[test]
fn plugin_remove_deletes_local_plugin_and_clears_persisted_state() {
  let mut context = RuntimeContext::new_in_memory();
  let storage_root = create_temp_workspace("plugin-remove-storage");
  let source_root =
    create_temp_plugin_bundle("plugin-remove-source", "focus-review", "Focus Review");
  let install_root = create_temp_workspace("plugin-remove-root");
  let store = FileThreadStore::new(
    storage_root.join("pith.db"),
    storage_root.join("threads.json"),
  );
  store
    .save_plugin_enabled("focus-review", true)
    .expect("save persisted plugin state");
  context.store = Some(store);
  context.plugin_roots = vec![install_root.clone()];
  context.plugin_install_root = install_root.clone();
  context.plugins = vec![];

  let install_response = handle_request(
    &mut context,
    request(
      methods::PLUGIN_INSTALL,
      Some(json!({
        "sourcePath": source_root.display().to_string()
      })),
    ),
  );
  assert!(install_response.error.is_none());

  let manifest_path = context.plugins[0].manifest_path.clone();
  let remove_response = handle_request(
    &mut context,
    request(
      methods::PLUGIN_REMOVE,
      Some(json!({
        "manifestPath": manifest_path
      })),
    ),
  );

  let persisted_states = context
    .store
    .as_ref()
    .expect("store")
    .load_plugin_states()
    .expect("load plugin states");

  fs::remove_dir_all(source_root.parent().expect("plugin source root"))
    .expect("cleanup plugin source root");
  fs::remove_dir_all(&install_root).expect("cleanup install root");
  fs::remove_dir_all(&storage_root).expect("cleanup storage root");

  assert!(remove_response.error.is_none());
  let result = remove_response.result.expect("plugin remove result");
  assert_eq!(result["pluginId"], "focus-review");
  assert!(context.plugins.is_empty());
  assert!(!persisted_states.contains_key("focus-review"));
}

#[test]
fn plugin_command_registry_lists_enabled_command_plugins() {
  let mut context = RuntimeContext::new_in_memory();
  context.plugins = vec![PluginCatalogEntry {
    id: "workspace-notes".to_string(),
    name: "workspace-notes".to_string(),
    version: "0.1.0".to_string(),
    display_name: "Workspace Notes".to_string(),
    status: "ready".to_string(),
    description: "Command-enabled plugin".to_string(),
    author_name: Some("Pith".to_string()),
    enabled: true,
    default_enabled: true,
    capabilities: vec![
      "command:workspace.capture-note".to_string(),
      "prompt_pack:workspace.notes".to_string(),
    ],
    permissions: vec!["file.read".to_string(), "file.write".to_string()],
    manifest_path: PathBuf::from(env!("CARGO_MANIFEST_DIR"))
      .join("../../plugins/bundled/workspace-notes/pith-plugin.json")
      .display()
      .to_string(),
    provenance: "bundled".to_string(),
    validation_error: None,
    validation_hint: None,
  }];

  let response = handle_request(
    &mut context,
    request(methods::PLUGIN_COMMAND_REGISTRY, None),
  );

  assert!(response.error.is_none());
  let result = response.result.expect("command registry result");
  let commands = result["commands"].as_array().expect("commands");
  assert_eq!(commands.len(), 1);
  assert_eq!(commands[0]["pluginId"], "workspace-notes");
  assert_eq!(commands[0]["title"], "Capture Workspace Note");
  assert_eq!(commands[0]["executionKind"], "builtin.workspaceReadmeNote");
}

#[test]
fn plugin_hook_registry_lists_enabled_hook_plugins() {
  let mut context = RuntimeContext::new_in_memory();
  context.plugins = vec![PluginCatalogEntry {
    id: "shell-recorder".to_string(),
    name: "shell-recorder".to_string(),
    version: "0.1.0".to_string(),
    display_name: "Shell Recorder".to_string(),
    status: "ready".to_string(),
    description: "Hook-enabled plugin".to_string(),
    author_name: Some("Pith".to_string()),
    enabled: true,
    default_enabled: false,
    capabilities: vec![
      "hook:shell.recorder".to_string(),
      "tool:shell.timeline".to_string(),
    ],
    permissions: vec!["shell.exec".to_string()],
    manifest_path: PathBuf::from(env!("CARGO_MANIFEST_DIR"))
      .join("../../plugins/bundled/shell-recorder/pith-plugin.json")
      .display()
      .to_string(),
    provenance: "bundled".to_string(),
    validation_error: None,
    validation_hint: None,
  }];

  let response = handle_request(&mut context, request(methods::PLUGIN_HOOK_REGISTRY, None));

  assert!(response.error.is_none());
  let result = response.result.expect("hook registry result");
  let hooks = result["hooks"].as_array().expect("hooks");
  assert_eq!(hooks.len(), 1);
  assert_eq!(hooks[0]["pluginId"], "shell-recorder");
  assert_eq!(hooks[0]["event"], "shell.completed");
  assert_eq!(hooks[0]["title"], "Record Shell Completion");
}

#[test]
fn plugin_connector_registry_lists_disabled_connector_plugins() {
  let mut context = RuntimeContext::new_in_memory();
  context.plugins = vec![PluginCatalogEntry {
    id: "notion-connector".to_string(),
    name: "notion-connector".to_string(),
    version: "0.1.0".to_string(),
    display_name: "Notion Connector".to_string(),
    status: "ready".to_string(),
    description: "Connector plugin".to_string(),
    author_name: Some("Pith".to_string()),
    enabled: false,
    default_enabled: false,
    capabilities: vec![
      "mcp_server:notion".to_string(),
      "connector:notion".to_string(),
    ],
    permissions: vec!["network.outbound".to_string(), "mcp.connect".to_string()],
    manifest_path: PathBuf::from(env!("CARGO_MANIFEST_DIR"))
      .join("../../plugins/bundled/notion-connector/pith-plugin.json")
      .display()
      .to_string(),
    provenance: "bundled".to_string(),
    validation_error: None,
    validation_hint: None,
  }];

  let response = handle_request(
    &mut context,
    request(methods::PLUGIN_CONNECTOR_REGISTRY, None),
  );

  assert!(response.error.is_none());
  let result = response.result.expect("connector registry result");
  let connectors = result["connectors"].as_array().expect("connectors");
  assert_eq!(connectors.len(), 1);
  assert_eq!(connectors[0]["connectorId"], "notion-connector::notion");
  assert_eq!(connectors[0]["status"], "disabled");
  assert_eq!(connectors[0]["authType"], "oauth2");
  assert_eq!(connectors[0]["credentialStore"], "keychain");
}

#[test]
fn plugin_command_run_executes_builtin_command_for_the_selected_thread() {
  let mut context = RuntimeContext::new_in_memory();
  let workspace = create_temp_workspace("plugin-command-run");
  context.plugins = vec![PluginCatalogEntry {
    id: "workspace-notes".to_string(),
    name: "workspace-notes".to_string(),
    version: "0.1.0".to_string(),
    display_name: "Workspace Notes".to_string(),
    status: "ready".to_string(),
    description: "Command-enabled plugin".to_string(),
    author_name: Some("Pith".to_string()),
    enabled: true,
    default_enabled: true,
    capabilities: vec![
      "command:workspace.capture-note".to_string(),
      "prompt_pack:workspace.notes".to_string(),
    ],
    permissions: vec!["file.read".to_string(), "file.write".to_string()],
    manifest_path: PathBuf::from(env!("CARGO_MANIFEST_DIR"))
      .join("../../plugins/bundled/workspace-notes/pith-plugin.json")
      .display()
      .to_string(),
    provenance: "bundled".to_string(),
    validation_error: None,
    validation_hint: None,
  }];
  fs::write(
    workspace.join("README.md"),
    "Workspace A\nCommand registry path\n",
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
        "title": "Plugin Command Thread"
      })),
    ),
  );

  let response = handle_request(
    &mut context,
    request(
      methods::PLUGIN_COMMAND_RUN,
      Some(json!({
        "threadId": "thread-1",
        "commandId": "workspace-notes::workspace.capture-note"
      })),
    ),
  );

  fs::remove_dir_all(&workspace).expect("cleanup temp workspace");

  assert!(response.error.is_none());
  let result = response.result.expect("command run result");
  let items = result["items"].as_array().expect("items");
  assert_eq!(items[0]["kind"], "pluginCommand");
  assert_eq!(items[0]["attributes"]["pluginId"], "workspace-notes");
  assert_eq!(items[1]["kind"], "pluginResult");
  assert_eq!(
    items[1]["attributes"]["executionKind"],
    "builtin.workspaceReadmeNote"
  );
  assert!(items[1]["content"]
    .as_str()
    .unwrap()
    .contains("Command registry path"));
  assert_eq!(items[2]["kind"], "assistantMessage");
  let memory_item = items
    .iter()
    .find(|item| item["title"] == "Memory Note Saved")
    .expect("memory note saved item");
  assert_eq!(memory_item["kind"], "system");
  assert_eq!(memory_item["attributes"]["pluginId"], "workspace-notes");
  assert_eq!(result["threadId"], "thread-1");
  assert_eq!(context.memory_notes.len(), 3);
  assert!(context
    .memory_notes
    .iter()
    .any(|note| note.title == "Workspace Capture" && note.source == "plugin.workspace-notes"));
}

#[test]
fn bundled_builtin_plugin_commands_return_owned_results() {
  let mut context = RuntimeContext::new_in_memory();
  let workspace = create_temp_workspace("bundled-plugin-results");
  fs::write(workspace.join("README.md"), "# Bundled Plugin Results\n").expect("write readme");
  context.plugins = vec![
    PluginCatalogEntry {
      id: "review-assistant".to_string(),
      name: "review-assistant".to_string(),
      version: "0.1.0".to_string(),
      display_name: "Review Assistant".to_string(),
      status: "ready".to_string(),
      description: "Review plugin".to_string(),
      author_name: Some("Pith".to_string()),
      enabled: true,
      default_enabled: true,
      capabilities: vec!["command:review.inspect-diff".to_string()],
      permissions: vec!["file.read".to_string(), "model.invoke".to_string()],
      manifest_path: PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../plugins/bundled/review-assistant/pith-plugin.json")
        .display()
        .to_string(),
      provenance: "bundled".to_string(),
      validation_error: None,
      validation_hint: None,
    },
    PluginCatalogEntry {
      id: "shell-recorder".to_string(),
      name: "shell-recorder".to_string(),
      version: "0.1.0".to_string(),
      display_name: "Shell Recorder".to_string(),
      status: "ready".to_string(),
      description: "Shell plugin".to_string(),
      author_name: Some("Pith".to_string()),
      enabled: true,
      default_enabled: false,
      capabilities: vec![
        "command:shell.summarize-session".to_string(),
        "hook:shell.recorder".to_string(),
      ],
      permissions: vec!["shell.exec".to_string()],
      manifest_path: PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../plugins/bundled/shell-recorder/pith-plugin.json")
        .display()
        .to_string(),
      provenance: "bundled".to_string(),
      validation_error: None,
      validation_hint: None,
    },
  ];

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
        "title": "Bundled Plugin Thread"
      })),
    ),
  );

  let review_response = handle_request(
    &mut context,
    request(
      methods::PLUGIN_COMMAND_RUN,
      Some(json!({
        "threadId": "thread-1",
        "commandId": "review-assistant::review.inspect-diff"
      })),
    ),
  );
  let shell_response = handle_request(
    &mut context,
    request(
      methods::PLUGIN_COMMAND_RUN,
      Some(json!({
        "threadId": "thread-1",
        "commandId": "shell-recorder::shell.summarize-session"
      })),
    ),
  );

  fs::remove_dir_all(&workspace).expect("cleanup temp workspace");

  assert!(review_response.error.is_none());
  assert!(shell_response.error.is_none());
  let review_result = review_response.result.expect("review result");
  let shell_result = shell_response.result.expect("shell result");
  let review_items = review_result["items"]
    .as_array()
    .expect("review items")
    .clone();
  let shell_items = shell_result["items"]
    .as_array()
    .expect("shell items")
    .clone();
  assert_eq!(review_items[1]["kind"], "pluginResult");
  assert_eq!(
    review_items[1]["attributes"]["executionKind"],
    "builtin.reviewDiffSummary"
  );
  assert_eq!(shell_items[1]["kind"], "pluginResult");
  assert_eq!(
    shell_items[1]["attributes"]["executionKind"],
    "builtin.shellSessionSummary"
  );
}

#[test]
fn plugin_command_run_rejects_commands_without_execution_contract() {
  let mut context = RuntimeContext::new_in_memory();
  let source_root =
    create_temp_plugin_bundle("plugin-command-contract", "prompt-only", "Prompt Only");
  let workspace = create_temp_workspace("plugin-command-contract-workspace");
  let plugin_manifest = source_root.join("pith-plugin.json");
  context.plugins = vec![PluginCatalogEntry {
    id: "prompt-only".to_string(),
    name: "prompt-only".to_string(),
    version: "0.1.0".to_string(),
    display_name: "Prompt Only".to_string(),
    status: "ready".to_string(),
    description: "Prompt-only command plugin".to_string(),
    author_name: Some("Pith".to_string()),
    enabled: true,
    default_enabled: true,
    capabilities: vec!["command:prompt-only.run".to_string()],
    permissions: vec!["file.read".to_string()],
    manifest_path: plugin_manifest.display().to_string(),
    provenance: "test".to_string(),
    validation_error: None,
    validation_hint: None,
  }];

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
        "title": "Plugin Contract Thread"
      })),
    ),
  );

  let response = handle_request(
    &mut context,
    request(
      methods::PLUGIN_COMMAND_RUN,
      Some(json!({
        "threadId": "thread-1",
        "commandId": "prompt-only::prompt-only.run"
      })),
    ),
  );

  fs::remove_dir_all(&workspace).expect("cleanup temp workspace");
  fs::remove_dir_all(source_root.parent().expect("plugin root")).expect("cleanup plugin source");

  let error = response.error.expect("command contract error");
  assert_eq!(error.code, -32053);
  assert!(error
    .message
    .contains("requires an explicit execution contract"));
}

#[test]
fn file_reads_require_plugin_permission() {
  let mut context = RuntimeContext::new_in_memory();
  let workspace = create_temp_workspace("permission-read");
  fs::write(workspace.join("README.md"), "# Permission Gate\n").expect("write readme");
  context.plugins = vec![PluginCatalogEntry {
    id: "shell-recorder".to_string(),
    name: "shell-recorder".to_string(),
    version: "0.1.0".to_string(),
    display_name: "Shell Recorder".to_string(),
    status: "ready".to_string(),
    description: "No file access".to_string(),
    author_name: Some("Pith".to_string()),
    enabled: false,
    default_enabled: false,
    capabilities: vec!["hook:shell.recorder".to_string()],
    permissions: vec!["shell.exec".to_string()],
    manifest_path: "plugins/bundled/shell-recorder/pith-plugin.json".to_string(),
    provenance: "bundled".to_string(),
    validation_error: None,
    validation_hint: None,
  }];

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
        "title": "Permission Thread"
      })),
    ),
  );

  let response = handle_request(
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

  assert!(response.error.is_none());
  let result = response.result.expect("turn result");
  let items = result["items"].as_array().expect("items");
  assert_eq!(items[2]["title"], "Plugin Permission Required");
  assert_eq!(items[2]["attributes"]["requiredPermission"], "file.read");
  assert_eq!(items[3]["kind"], "assistantMessage");
}

#[test]
fn shell_requests_require_plugin_permission_before_approval() {
  let mut context = RuntimeContext::new_in_memory();
  let workspace = create_temp_workspace("permission-shell");
  context.plugins = vec![PluginCatalogEntry {
    id: "workspace-notes".to_string(),
    name: "workspace-notes".to_string(),
    version: "0.1.0".to_string(),
    display_name: "Workspace Notes".to_string(),
    status: "ready".to_string(),
    description: "No shell access".to_string(),
    author_name: Some("Pith".to_string()),
    enabled: true,
    default_enabled: true,
    capabilities: vec!["prompt_pack:workspace.notes".to_string()],
    permissions: vec!["file.read".to_string(), "file.write".to_string()],
    manifest_path: "plugins/bundled/workspace-notes/pith-plugin.json".to_string(),
    provenance: "bundled".to_string(),
    validation_error: None,
    validation_hint: None,
  }];

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
        "title": "Shell Permission Thread"
      })),
    ),
  );

  let response = handle_request(
    &mut context,
    request(
      methods::TURN_START,
      Some(json!({
        "threadId": "thread-1",
        "message": "Run shell: ls"
      })),
    ),
  );

  fs::remove_dir_all(&workspace).expect("cleanup temp workspace");

  assert!(response.error.is_none());
  let result = response.result.expect("turn result");
  let items = result["items"].as_array().expect("items");
  assert_eq!(items[2]["title"], "Plugin Permission Required");
  assert_eq!(items[2]["attributes"]["requiredPermission"], "shell.exec");
  assert!(result["pendingApprovals"]
    .as_array()
    .expect("pending approvals")
    .is_empty());
}

#[test]
fn approval_resolution_rechecks_plugin_permissions() {
  let mut context = RuntimeContext::new_in_memory();
  let workspace = create_temp_workspace("approval-permission-recheck");
  context.plugins = vec![PluginCatalogEntry {
    id: "workspace-notes".to_string(),
    name: "workspace-notes".to_string(),
    version: "0.1.0".to_string(),
    display_name: "Workspace Notes".to_string(),
    status: "ready".to_string(),
    description: "Write access plugin".to_string(),
    author_name: Some("Pith".to_string()),
    enabled: true,
    default_enabled: true,
    capabilities: vec!["prompt_pack:workspace.notes".to_string()],
    permissions: vec!["file.read".to_string(), "file.write".to_string()],
    manifest_path: "plugins/bundled/workspace-notes/pith-plugin.json".to_string(),
    provenance: "bundled".to_string(),
    validation_error: None,
    validation_hint: None,
  }];

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
        "title": "Approval Permission Thread"
      })),
    ),
  );

  let turn_response = handle_request(
    &mut context,
    request(
      methods::TURN_START,
      Some(json!({
        "threadId": "thread-1",
        "message": "Write docs/output.txt: gated content"
      })),
    ),
  );
  let approval_id = turn_response.result.expect("turn result")["pendingApprovals"][0]["id"]
    .as_str()
    .expect("approval id")
    .to_string();

  context.plugins[0].enabled = false;

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

  let written_file = workspace.join("docs").join("output.txt");
  fs::remove_dir_all(&workspace).expect("cleanup temp workspace");

  assert!(approval_response.error.is_none());
  let approval_result = approval_response.result.expect("approval result");
  let items = approval_result["items"].as_array().expect("approval items");
  assert_eq!(items[1]["title"], "Plugin Permission Required");
  assert_eq!(items[1]["attributes"]["requiredPermission"], "file.write");
  assert!(!written_file.exists());
}

#[test]
fn capability_registry_only_includes_ready_enabled_plugins() {
  let plugins = vec![
    PluginCatalogEntry {
      id: "workspace-notes".to_string(),
      name: "workspace-notes".to_string(),
      version: "0.1.0".to_string(),
      display_name: "Workspace Notes".to_string(),
      status: "ready".to_string(),
      description: "Test plugin".to_string(),
      author_name: Some("Pith".to_string()),
      enabled: true,
      default_enabled: true,
      capabilities: vec![
        "prompt_pack:workspace.notes".to_string(),
        "settings:workspace.preferences".to_string(),
      ],
      permissions: vec!["file.read".to_string(), "file.write".to_string()],
      manifest_path: "plugins/bundled/workspace-notes/pith-plugin.json".to_string(),
      provenance: "bundled".to_string(),
      validation_error: None,
      validation_hint: None,
    },
    PluginCatalogEntry {
      id: "shell-recorder".to_string(),
      name: "shell-recorder".to_string(),
      version: "0.1.0".to_string(),
      display_name: "Shell Recorder".to_string(),
      status: "ready".to_string(),
      description: "Disabled plugin".to_string(),
      author_name: Some("Pith".to_string()),
      enabled: false,
      default_enabled: false,
      capabilities: vec!["hook:shell.recorder".to_string()],
      permissions: vec!["shell.exec".to_string()],
      manifest_path: "plugins/bundled/shell-recorder/pith-plugin.json".to_string(),
      provenance: "bundled".to_string(),
      validation_error: None,
      validation_hint: None,
    },
    PluginCatalogEntry {
      id: "broken-plugin".to_string(),
      name: "broken-plugin".to_string(),
      version: "0.1.0".to_string(),
      display_name: "Broken Plugin".to_string(),
      status: "invalid".to_string(),
      description: "Invalid plugin".to_string(),
      author_name: None,
      enabled: false,
      default_enabled: false,
      capabilities: vec![],
      permissions: vec![],
      manifest_path: "plugins/bundled/broken/pith-plugin.json".to_string(),
      provenance: "bundled".to_string(),
      validation_error: Some("plugin capability kind `memory` is not supported".to_string()),
      validation_hint: Some(
        "Use one of the supported capability kinds: command, agent, prompt_pack, hook, tool, mcp_server, skill, connector, settings.".to_string(),
      ),
    },
  ];

  let result = build_protocol_capability_registry(&plugins);

  assert_eq!(result.summary.enabled_plugin_count, 1);
  assert_eq!(result.summary.total_capability_count, 2);
  assert_eq!(
    result.summary.capability_counts_by_kind.get("prompt_pack"),
    Some(&1)
  );
  assert_eq!(
    result.summary.capability_counts_by_kind.get("settings"),
    Some(&1)
  );
  assert_eq!(result.capabilities.len(), 2);
  assert_eq!(result.capabilities[0].kind, "prompt_pack");
  assert_eq!(result.capabilities[0].plugin_id, "workspace-notes");
  assert_eq!(result.capabilities[1].kind, "settings");
}
