use super::test_support::{
  create_temp_workspace, enable_full_access_plugin, remove_temp_workspace, request,
};
use super::*;
use amentia_protocol::methods;
use serde_json::json;
use std::fs;

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

  remove_temp_workspace(&workspace_a);
  remove_temp_workspace(&workspace_b);

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
fn thread_delete_removes_session_without_touching_workspace_files() {
  let mut context = RuntimeContext::new_in_memory();
  let workspace = create_temp_workspace("thread-delete");
  let workspace_file = workspace.join("notes.txt");
  fs::write(&workspace_file, "keep workspace data\n").expect("write workspace file");

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
        "title": "Disposable Session"
      })),
    ),
  );

  let delete_response = handle_request(
    &mut context,
    request(
      methods::THREAD_DELETE,
      Some(json!({
        "threadId": "thread-1"
      })),
    ),
  );
  let list_response = handle_request(&mut context, request(methods::THREAD_LIST, None));

  let file_content = fs::read_to_string(&workspace_file).expect("workspace file remains readable");
  remove_temp_workspace(&workspace);

  assert!(delete_response.error.is_none());
  let delete_result = delete_response.result.expect("delete result");
  assert_eq!(delete_result["threadId"], "thread-1");
  assert_eq!(delete_result["deleted"], true);
  assert_eq!(
    delete_result["threads"]
      .as_array()
      .expect("delete result threads")
      .len(),
    0
  );
  assert_eq!(
    list_response.result.expect("list result")["threads"]
      .as_array()
      .expect("listed threads")
      .len(),
    0
  );
  assert_eq!(file_content, "keep workspace data\n");
}

#[test]
fn thread_revert_without_changes_returns_plain_noop_receipt() {
  let mut context = RuntimeContext::new_in_memory();

  let _ = handle_request(
    &mut context,
    request(
      methods::THREAD_START,
      Some(json!({
        "title": "Clean Session"
      })),
    ),
  );

  let revert_response = handle_request(
    &mut context,
    request(
      methods::THREAD_REVERT_CHANGES,
      Some(json!({
        "threadId": "thread-1"
      })),
    ),
  );

  assert!(revert_response.error.is_none());
  let result = revert_response.result.expect("revert result");
  assert_eq!(result["revertedCount"], 0);
  let items = result["items"].as_array().expect("revert items");
  assert_eq!(items[0]["title"], "No Session Changes To Revert");
  assert_eq!(
    items[0]["content"],
    "This session has no saved project files left to revert."
  );
  assert_eq!(items[0]["attributes"]["receiptKind"], "sessionChangeRevert");
}
