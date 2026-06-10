use super::test_support::{create_temp_workspace, enable_full_access_plugin, request};
use super::*;
use pith_protocol::methods;
use pith_storage::RuntimeStore;
use serde_json::json;
use std::fs;

#[test]
fn approval_respond_writes_file_after_approval() {
  let mut context = RuntimeContext::new_in_memory();
  enable_full_access_plugin(&mut context);
  let workspace = create_temp_workspace("approval-write");
  let store_root = create_temp_workspace("approval-write-store");
  let store = RuntimeStore::new(store_root.join("pith.db"), store_root.join("threads.json"));
  context
    .persistence_state
    .set_store_for_testing(store.clone());

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
  let approval_step_id = turn_items[4]["attributes"]["agentStepId"]
    .as_str()
    .expect("approval step id")
    .to_string();
  let approval_loop_id = turn_items[4]["attributes"]["agentLoopId"]
    .as_str()
    .expect("approval loop id")
    .to_string();
  let approval_tool_call_id = turn_items[2]["attributes"]["toolCallId"]
    .as_str()
    .expect("approval tool call id")
    .to_string();
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
  let workspace_changes = store
    .load_workspace_changes_for_thread("thread-1")
    .expect("load workspace changes");
  let preview_response = handle_request(
    &mut context,
    request(
      methods::THREAD_CHANGE_PREVIEW,
      Some(json!({
        "threadId": "thread-1"
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
  let reverted_changes = store
    .load_workspace_changes_for_thread("thread-1")
    .expect("load reverted workspace changes");
  let file_exists_after_revert = workspace.join("docs").join("output.txt").exists();
  fs::remove_dir_all(&workspace).expect("cleanup temp workspace");
  fs::remove_dir_all(&store_root).expect("cleanup temp store");

  assert!(approval_response.error.is_none());
  let approval_result = approval_response.result.expect("approval result");
  let items = approval_result["items"].as_array().expect("approval items");

  assert_eq!(items[0]["kind"], "approvalResolved");
  assert_eq!(
    items[0]["attributes"]["agentStepId"].as_str(),
    Some(approval_step_id.as_str())
  );
  assert_eq!(
    items[0]["attributes"]["agentLoopId"].as_str(),
    Some(approval_loop_id.as_str())
  );
  assert_eq!(items[0]["attributes"]["agentStepPhase"], "approvalResume");
  assert_eq!(items[0]["attributes"]["agentStepStatus"], "completed");
  assert_eq!(items[0]["attributes"]["agentStepResume"], "true");
  assert_eq!(items[1]["title"], "write_file");
  assert_eq!(items[1]["attributes"]["tool"], "write_file");
  assert_eq!(items[1]["attributes"]["relativePath"], "docs/output.txt");
  assert_eq!(items[1]["attributes"]["maxBytes"], "1048576");
  assert_eq!(
    items[1]["attributes"]["agentStepId"].as_str(),
    Some(approval_step_id.as_str())
  );
  assert_eq!(items[1]["attributes"]["agentStepPhase"], "toolCall");
  assert_eq!(
    items[1]["attributes"]["toolCallId"].as_str(),
    Some(approval_tool_call_id.as_str())
  );
  assert_eq!(items[1]["attributes"]["toolCallStatus"], "started");
  assert_eq!(items[2]["attributes"]["tool"], "write_file");
  assert_eq!(items[2]["attributes"]["bytesWritten"], "26");
  assert_eq!(items[2]["attributes"]["maxBytes"], "1048576");
  assert_eq!(
    items[2]["attributes"]["agentStepId"].as_str(),
    Some(approval_step_id.as_str())
  );
  assert_eq!(items[2]["attributes"]["agentStepPhase"], "observation");
  assert_eq!(
    items[2]["attributes"]["toolCallId"].as_str(),
    Some(approval_tool_call_id.as_str())
  );
  assert_eq!(items[2]["attributes"]["toolCallStatus"], "completed");
  assert_eq!(items[3]["kind"], "assistantMessage");
  assert_eq!(items[3]["attributes"]["handoffKind"], "approvedWrite");
  assert_eq!(items[3]["attributes"]["responseRole"], "actionHandoff");
  assert_eq!(items[3]["attributes"]["relativePath"], "docs/output.txt");
  assert_eq!(items[3]["attributes"]["bytesWritten"], "26");
  assert_eq!(items[3]["attributes"]["continuationKind"], "fileSaved");
  assert!(items[3]["attributes"]["continuationSuggestion"]
    .as_str()
    .expect("continuation suggestion")
    .contains("continue from the saved change"));
  assert_eq!(
    items[3]["attributes"]["agentLoopSuccessfulObservationCount"],
    "1"
  );
  assert_eq!(items[3]["attributes"]["agentLoopFailureCount"], "0");
  assert_eq!(written_content, "Approval protected content");
  assert_eq!(workspace_changes.len(), 1);
  assert_eq!(workspace_changes[0].thread_id, "thread-1");
  assert_eq!(
    workspace_changes[0].approval_id.as_deref(),
    Some("approval-1")
  );
  assert_eq!(workspace_changes[0].relative_path, "docs/output.txt");
  assert_eq!(workspace_changes[0].previous_content, None);
  assert_eq!(
    workspace_changes[0].next_content,
    b"Approval protected content".to_vec()
  );
  assert!(preview_response.error.is_none());
  let preview_result = preview_response.result.expect("preview result");
  assert_eq!(preview_result["changes"].as_array().unwrap().len(), 1);
  assert_eq!(preview_result["changes"][0]["relativePath"], "docs/output.txt");
  assert_eq!(preview_result["changes"][0]["willDeleteFile"], true);
  assert!(revert_response.error.is_none());
  let revert_result = revert_response.result.expect("revert result");
  assert_eq!(revert_result["revertedCount"], 1);
  assert!(!file_exists_after_revert);
  assert!(reverted_changes[0].reverted_at.is_some());
}

#[test]
fn natural_handoff_save_uses_write_approval() {
  let mut context = RuntimeContext::new_in_memory();
  enable_full_access_plugin(&mut context);
  let workspace = create_temp_workspace("approval-handoff-save");

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
        "title": "Handoff Save Thread"
      })),
    ),
  );

  let turn_response = handle_request(
    &mut context,
    request(
      methods::TURN_START,
      Some(json!({
        "threadId": "thread-1",
        "message": "Save handoff to docs/handoff.md: Ship M7 carefully."
      })),
    ),
  );

  assert!(turn_response.error.is_none());
  let turn_result = turn_response.result.expect("turn result");
  let turn_items = turn_result["items"].as_array().expect("turn items");
  assert_eq!(turn_items[3]["kind"], "diffArtifact");
  assert_eq!(
    turn_items[3]["attributes"]["relativePath"],
    "docs/handoff.md"
  );
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
    fs::read_to_string(workspace.join("docs").join("handoff.md")).expect("read handoff");
  fs::remove_dir_all(&workspace).expect("cleanup temp workspace");

  assert!(approval_response.error.is_none());
  assert_eq!(written_content, "Ship M7 carefully.");
  let approval_result = approval_response.result.expect("approval result");
  let approval_items = approval_result["items"].as_array().expect("approval items");
  assert_eq!(
    approval_items[3]["attributes"]["handoffKind"],
    "approvedWrite"
  );
  assert_eq!(
    approval_items[3]["attributes"]["continuationKind"],
    "handoffSaved"
  );
  assert!(approval_items[3]["content"]
    .as_str()
    .expect("handoff content")
    .contains("prepare a connector update"));
}

#[test]
fn approved_workspace_execution_writes_without_pending_approval() {
  let mut context = RuntimeContext::new_in_memory();
  enable_full_access_plugin(&mut context);
  let workspace = create_temp_workspace("auto-write");

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
        "title": "Auto Write Thread"
      })),
    ),
  );

  let turn_response = handle_request(
    &mut context,
    request(
      methods::TURN_START,
      Some(json!({
        "threadId": "thread-1",
        "message": "Write docs/auto.txt: Auto approved content",
        "localExecutionSafetyMode": "approvedWorkspaceExecution"
      })),
    ),
  );

  let written_content =
    fs::read_to_string(workspace.join("docs").join("auto.txt")).expect("read written output");
  fs::remove_dir_all(&workspace).expect("cleanup temp workspace");

  assert!(turn_response.error.is_none());
  let turn_result = turn_response.result.expect("turn result");
  assert_eq!(turn_result["pendingApprovals"].as_array().unwrap().len(), 0);
  let items = turn_result["items"].as_array().expect("turn items");
  assert!(items.iter().any(|item| {
    item["title"] == "write_file result"
      && item["attributes"]["actionApprovalPolicy"] == "autoApproved"
      && item["attributes"]["localExecutionSafetyMode"] == "approvedWorkspaceExecution"
  }));
  assert!(items.iter().any(|item| {
    item["attributes"]["handoffKind"] == "autoApprovedWrite"
      && item["attributes"]["relativePath"] == "docs/auto.txt"
  }));
  assert_eq!(written_content, "Auto approved content");
}

#[test]
fn explore_mode_blocks_workspace_write_even_with_permission() {
  let mut context = RuntimeContext::new_in_memory();
  enable_full_access_plugin(&mut context);
  let workspace = create_temp_workspace("explore-write");

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
        "title": "Explore Write Thread"
      })),
    ),
  );

  let turn_response = handle_request(
    &mut context,
    request(
      methods::TURN_START,
      Some(json!({
        "threadId": "thread-1",
        "message": "Write docs/explore.txt: This should not be written",
        "localExecutionSafetyMode": "explore"
      })),
    ),
  );

  let target_exists = workspace.join("docs").join("explore.txt").exists();
  fs::remove_dir_all(&workspace).expect("cleanup temp workspace");

  assert!(turn_response.error.is_none());
  let turn_result = turn_response.result.expect("turn result");
  assert_eq!(turn_result["pendingApprovals"].as_array().unwrap().len(), 0);
  assert!(!target_exists);
  let items = turn_result["items"].as_array().expect("turn items");
  assert!(items.iter().any(|item| {
    item["attributes"]["localExecutionSafetyMode"] == "explore"
      && item["attributes"]["actionApprovalPolicy"] == "blocked"
      && item["attributes"]["blockReason"] == "readOnlyMode"
      && item["attributes"]["requiredPermission"] == "file.write"
      && item["attributes"]["retryMessage"] == "Write docs/explore.txt: This should not be written"
  }));
}
