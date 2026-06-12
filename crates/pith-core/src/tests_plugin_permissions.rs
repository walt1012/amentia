use super::test_support::{
  bundled_plugin_entry, create_temp_workspace, replace_plugin_catalog, request,
};
use super::*;
use pith_protocol::methods;
use serde_json::json;
use std::fs;

#[test]
fn file_reads_require_plugin_permission() {
  let mut context = RuntimeContext::new_in_memory();
  let workspace = create_temp_workspace("permission-read");
  fs::write(workspace.join("README.md"), "# Permission Gate\n").expect("write readme");
  replace_plugin_catalog(
    &mut context,
    vec![bundled_plugin_entry(
      "shell-recorder",
      "Shell Recorder",
      false,
      false,
      &["hook:shell.recorder"],
      &["shell.exec"],
    )],
  );

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
  assert_eq!(items[2]["title"], "Connector Permission Required");
  assert_eq!(items[2]["attributes"]["requiredPermission"], "file.read");
  assert_eq!(items[3]["kind"], "assistantMessage");
}

#[test]
fn shell_requests_require_plugin_permission_before_approval() {
  let mut context = RuntimeContext::new_in_memory();
  let workspace = create_temp_workspace("permission-shell");
  replace_plugin_catalog(
    &mut context,
    vec![bundled_plugin_entry(
      "workspace-notes",
      "Workspace Notes",
      true,
      true,
      &["prompt_pack:workspace.notes"],
      &["file.read", "file.write"],
    )],
  );

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
  assert_eq!(items[2]["title"], "Connector Permission Required");
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
  replace_plugin_catalog(
    &mut context,
    vec![bundled_plugin_entry(
      "workspace-notes",
      "Workspace Notes",
      true,
      true,
      &["prompt_pack:workspace.notes"],
      &["file.read", "file.write"],
    )],
  );

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

  context
    .plugin_state
    .set_enabled("workspace-notes", false)
    .expect("disable plugin");

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
  assert_eq!(items[1]["title"], "Connector Permission Required");
  assert_eq!(items[1]["attributes"]["requiredPermission"], "file.write");
  assert!(!written_file.exists());
}
