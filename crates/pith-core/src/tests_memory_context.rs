use super::test_support::{create_temp_workspace, enable_full_access_plugin, request};
use super::*;
use pith_protocol::methods;
use serde_json::json;
use std::fs;

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
