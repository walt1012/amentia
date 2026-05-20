use super::test_support::{create_temp_workspace, enable_full_access_plugin, request};
use super::*;
use pith_protocol::methods;
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
