use super::test_support::{
  bundled_manifest_plugin_entry, create_temp_workspace, enable_full_access_plugin,
  replace_plugin_catalog, request,
};
use super::*;
use pith_protocol::methods;
use serde_json::json;
use std::fs;

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
  let cancel_result = cancel_response.result.expect("cancel result");
  assert_eq!(cancel_result["threadId"], "thread-1");
  assert_eq!(cancel_result["turnId"], serde_json::Value::Null);

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
