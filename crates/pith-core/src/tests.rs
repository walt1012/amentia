use super::test_support::{create_temp_workspace, enable_full_access_plugin, request};
use super::*;
use pith_protocol::methods;
use serde_json::json;
use std::fs;
use std::thread;

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
