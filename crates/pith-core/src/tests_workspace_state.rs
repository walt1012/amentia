use super::test_support::{create_temp_workspace, request};
use super::*;
use pith_protocol::methods;
use serde_json::json;
use std::fs;

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
