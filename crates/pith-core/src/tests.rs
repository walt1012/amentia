use super::test_support::{
  bundled_manifest_plugin_entry, create_temp_workspace, enable_full_access_plugin,
  replace_plugin_catalog, request,
};
use super::*;
use pith_protocol::methods;
use serde_json::json;
use std::fs;
use std::thread;

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
  assert_eq!(context.memory_state.note_count(), 2);
  assert_eq!(
    context
      .memory_state
      .latest_note()
      .expect("latest note")
      .title,
    "Repository preference"
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
fn turn_start_web_search_uses_builtin_network_permission() {
  let mut context = RuntimeContext::new_in_memory();
  replace_plugin_catalog(&mut context, vec![]);

  let _ = handle_request(
    &mut context,
    request(
      methods::THREAD_START,
      Some(json!({
        "title": "Web Thread"
      })),
    ),
  );
  let _ = handle_request(
    &mut context,
    request(
      methods::TURN_CANCEL_RUNNING,
      Some(json!({
        "threadId": "thread-1"
      })),
    ),
  );

  let turn_response = handle_request(
    &mut context,
    request(
      methods::TURN_START,
      Some(json!({
        "threadId": "thread-1",
        "message": "web search for Pith local model"
      })),
    ),
  );

  assert!(turn_response.error.is_none());
  let result = turn_response.result.expect("turn result");
  let items = result["items"].as_array().expect("items");

  assert_eq!(items[0]["kind"], "userMessage");
  assert_eq!(items[1]["kind"], "plan");
  assert_eq!(items[2]["kind"], "warning");
  assert_eq!(items[2]["title"], "Turn Cancelled");
  assert!(items
    .iter()
    .all(|item| item["title"] != "Plugin Permission Required"));
}

#[test]
fn turn_start_routes_fresh_public_requests_to_builtin_web_search() {
  let mut context = RuntimeContext::new_in_memory();
  replace_plugin_catalog(&mut context, vec![]);

  let _ = handle_request(
    &mut context,
    request(
      methods::THREAD_START,
      Some(json!({
        "title": "Fresh Info Thread"
      })),
    ),
  );
  let _ = handle_request(
    &mut context,
    request(
      methods::TURN_CANCEL_RUNNING,
      Some(json!({
        "threadId": "thread-1"
      })),
    ),
  );

  let turn_response = handle_request(
    &mut context,
    request(
      methods::TURN_START,
      Some(json!({
        "threadId": "thread-1",
        "message": "What is the latest LFM2.5 release?"
      })),
    ),
  );

  assert!(turn_response.error.is_none());
  let result = turn_response.result.expect("turn result");
  let items = result["items"].as_array().expect("items");

  assert_eq!(items[0]["kind"], "userMessage");
  assert_eq!(items[1]["kind"], "plan");
  assert_eq!(items[2]["kind"], "warning");
  assert_eq!(items[2]["title"], "Turn Cancelled");
  assert!(items
    .iter()
    .all(|item| item["title"] != "Plugin Permission Required"));
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
  assert_eq!(items[2]["attributes"]["tool"], "read_file");
  assert_eq!(items[2]["attributes"]["relativePath"], "README.md");
  assert_eq!(items[2]["attributes"]["maxBytes"], "4096");
  assert_eq!(
    items[2]["attributes"]["workspaceDisplayName"],
    workspace.file_name().unwrap().to_str().unwrap()
  );
  assert_eq!(items[3]["kind"], "toolResult");
  assert_eq!(items[3]["attributes"]["tool"], "read_file");
  assert_eq!(items[3]["attributes"]["maxBytes"], "4096");
  assert_eq!(items[3]["attributes"]["isTruncated"], "false");
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
fn turn_start_prefers_workspace_file_over_fresh_web_search() {
  let mut context = RuntimeContext::new_in_memory();
  enable_full_access_plugin(&mut context);
  let workspace = create_temp_workspace("fresh-local-file");
  fs::write(
    workspace.join("Cargo.toml"),
    "[package]\nname = \"pith-test\"\nversion = \"0.1.0\"\n",
  )
  .expect("write cargo manifest");

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
        "title": "Local Manifest Thread"
      })),
    ),
  );

  let turn_response = handle_request(
    &mut context,
    request(
      methods::TURN_START,
      Some(json!({
        "threadId": "thread-1",
        "message": "What version is in Cargo.toml?"
      })),
    ),
  );

  fs::remove_dir_all(&workspace).expect("cleanup temp workspace");

  assert!(turn_response.error.is_none());
  let result = turn_response.result.expect("turn result");
  let items = result["items"].as_array().expect("items");

  assert_eq!(items[2]["kind"], "toolStart");
  assert_eq!(items[2]["title"], "read_file");
  assert_eq!(items[2]["attributes"]["tool"], "read_file");
  assert_eq!(items[2]["attributes"]["relativePath"], "Cargo.toml");
  assert_eq!(items[3]["kind"], "toolResult");
  assert!(items[3]["content"].as_str().unwrap().contains("0.1.0"));
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
  assert_eq!(items[2]["attributes"]["tool"], "search_files");
  assert_eq!(items[2]["attributes"]["query"], "Search target");
  assert_eq!(items[2]["attributes"]["maxResults"], "12");
  assert_eq!(items[3]["kind"], "toolResult");
  assert_eq!(items[3]["attributes"]["resultCount"], "2");
  assert_eq!(items[3]["attributes"]["maxResults"], "12");
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
  assert_eq!(turn_items[2]["attributes"]["tool"], "generate_diff");
  assert_eq!(turn_items[2]["attributes"]["maxBytes"], "131072");
  assert_eq!(
    turn_items[2]["attributes"]["relativePath"],
    "docs/output.txt"
  );
  assert_eq!(turn_items[3]["kind"], "diffArtifact");
  assert_eq!(turn_items[3]["attributes"]["tool"], "generate_diff");
  assert_eq!(turn_items[3]["attributes"]["maxBytes"], "131072");
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
  assert_eq!(items[1]["attributes"]["tool"], "write_file");
  assert_eq!(items[1]["attributes"]["relativePath"], "docs/output.txt");
  assert_eq!(items[1]["attributes"]["maxBytes"], "1048576");
  assert_eq!(items[2]["attributes"]["tool"], "write_file");
  assert_eq!(items[2]["attributes"]["bytesWritten"], "26");
  assert_eq!(items[2]["attributes"]["maxBytes"], "1048576");
  assert_eq!(written_content, "Approval protected content");
}

#[test]
fn approval_respond_runs_shell_after_approval() {
  let mut context = RuntimeContext::new_in_memory();
  let workspace = create_temp_workspace("approval-shell");
  replace_plugin_catalog(
    &mut context,
    vec![bundled_manifest_plugin_entry(
      "shell-recorder",
      "Shell Recorder",
      true,
      false,
      &["hook:shell.recorder", "tool:shell.timeline"],
      &["shell.exec"],
    )],
  );
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
  assert_eq!(items[1]["attributes"]["tool"], "run_shell");
  assert_eq!(items[1]["attributes"]["command"], "ls");
  assert!(items[2]["content"].as_str().unwrap().contains("marker.txt"));
  assert_eq!(items[2]["attributes"]["tool"], "run_shell");
  assert_eq!(items[2]["attributes"]["command"], "ls");
  assert_eq!(items[2]["attributes"]["exitCode"], "0");
  assert_eq!(items[2]["attributes"]["sandboxMode"], "workspaceReadWrite");
  assert!(items[2]["attributes"]["sandboxBackend"].is_string());
  assert!(items[2]["attributes"]["sandboxActive"].is_string());
  assert!(items[2]["attributes"]["sandboxNetworkAllowed"].is_string());
  assert_eq!(
    items[2]["attributes"]["sandboxOutputContextMode"],
    "sandboxOutputPreview"
  );
  assert_eq!(
    items[3]["attributes"]["sandboxOutputContextMode"],
    "sandboxOutputPreview"
  );
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
    .memory_state
    .recent_notes(16)
    .into_iter()
    .any(|note| note.title == "Shell Completion" && note.source == "plugin.shell-recorder"));
}

#[test]
fn approved_shell_execution_honors_pending_cancellation() {
  let mut context = RuntimeContext::new_in_memory();
  enable_full_access_plugin(&mut context);
  let workspace = create_temp_workspace("approval-shell-cancel");

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
        "title": "Shell Cancel Thread"
      })),
    ),
  );

  let turn_response = handle_request(
    &mut context,
    request(
      methods::TURN_START,
      Some(json!({
        "threadId": "thread-1",
        "message": "Run shell: sleep 5"
      })),
    ),
  );
  let approval_id = turn_response.result.expect("turn result")["pendingApprovals"][0]["id"]
    .as_str()
    .expect("approval id")
    .to_string();

  let cancel_response = handle_request(
    &mut context,
    request(
      methods::TURN_CANCEL_RUNNING,
      Some(json!({
        "threadId": "thread-1"
      })),
    ),
  );
  assert!(cancel_response.error.is_none());

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
  let shell_result = items
    .iter()
    .find(|item| item["title"] == "run_shell result")
    .expect("shell result");

  assert_eq!(shell_result["attributes"]["cancelled"], "true");
  assert_eq!(shell_result["attributes"]["exitCode"], "-1");
  assert!(shell_result["content"]
    .as_str()
    .expect("shell content")
    .contains("command cancelled"));

  let readiness_response = handle_request(&mut context, request(methods::RUNTIME_READINESS, None));
  let readiness = readiness_response.result.expect("readiness result");
  assert_eq!(readiness["metrics"]["runningApprovalCount"], "0");
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
    .memory_state
    .recent_notes(16)
    .into_iter()
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

  assert_eq!(items[1]["attributes"]["memoryNoteCount"], "2");
  assert_eq!(items[1]["attributes"]["memoryContextWindowTokens"], "4096");
  assert_eq!(items[1]["attributes"]["memoryContextBudgetChars"], "1228");
  assert!(items[1]["attributes"]["memoryNoteTitles"]
    .as_str()
    .unwrap()
    .contains("Wrote docs/output.txt"));
  assert!(items[4]["attributes"]["memoryNoteTitles"]
    .as_str()
    .unwrap()
    .contains("Wrote docs/output.txt"));
  assert_eq!(items[4]["attributes"]["memoryNoteCount"], "2");
  assert_eq!(items[4]["attributes"]["memoryContextWindowTokens"], "4096");
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
