use super::test_support::{
  create_temp_workspace, enable_full_access_plugin, replace_plugin_catalog, request,
};
use super::*;
use pith_protocol::methods;
use serde_json::json;
use std::fs;

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
