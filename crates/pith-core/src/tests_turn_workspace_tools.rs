use super::test_support::{create_temp_workspace, enable_full_access_plugin, request};
use super::*;
use pith_protocol::methods;
use serde_json::json;
use std::fs;

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
  assert_eq!(
    items[1]["attributes"]["agentStepId"],
    "thread-1-turn-1-step-1"
  );
  assert_eq!(
    items[1]["attributes"]["agentLoopId"],
    "thread-1-turn-1-loop-1"
  );
  assert_eq!(items[1]["attributes"]["agentLoopMaxSteps"], "3");
  assert_eq!(items[1]["attributes"]["agentLoopMode"], "dispatcherLoop");
  assert_eq!(items[1]["attributes"]["agentLoopStepCount"], "1");
  assert_eq!(items[1]["attributes"]["agentLoopBudgetRemaining"], "2");
  assert_eq!(items[1]["attributes"]["agentLoopStopReason"], "streaming");
  assert_eq!(items[1]["attributes"]["agentStepPhase"], "plan");
  assert_eq!(items[1]["attributes"]["agentToolName"], "read_file");
  assert_eq!(items[1]["attributes"]["responseRole"], "planner");
  assert_eq!(items[1]["attributes"]["memoryNoteCount"], "1");
  assert_eq!(items[1]["attributes"]["memoryNoteIds"], "memory-1");
  assert_eq!(items[2]["kind"], "toolStart");
  assert_eq!(items[2]["attributes"]["agentStepPhase"], "toolCall");
  assert_eq!(
    items[2]["attributes"]["toolCallId"],
    "thread-1-turn-1-step-1-tool-1"
  );
  assert_eq!(items[2]["attributes"]["toolCallStatus"], "started");
  assert_eq!(items[2]["attributes"]["tool"], "read_file");
  assert_eq!(items[2]["attributes"]["relativePath"], "README.md");
  assert_eq!(items[2]["attributes"]["maxBytes"], "4096");
  assert_eq!(
    items[2]["attributes"]["workspaceDisplayName"],
    workspace.file_name().unwrap().to_str().unwrap()
  );
  assert_eq!(items[3]["kind"], "toolResult");
  assert_eq!(items[3]["attributes"]["agentStepPhase"], "observation");
  assert_eq!(items[3]["attributes"]["toolCallStatus"], "completed");
  assert_eq!(items[3]["attributes"]["tool"], "read_file");
  assert_eq!(items[3]["attributes"]["maxBytes"], "4096");
  assert_eq!(items[3]["attributes"]["isTruncated"], "false");
  assert_eq!(items[4]["kind"], "assistantMessage");
  assert_eq!(items[4]["attributes"]["agentStepStatus"], "streaming");
  assert_eq!(items[4]["attributes"]["agentStepPhase"], "final");
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
  assert_eq!(items[2]["attributes"]["agentToolKind"], "workspace");
  assert_eq!(items[2]["attributes"]["agentToolName"], "search_files");
  assert_eq!(items[2]["attributes"]["tool"], "search_files");
  assert_eq!(items[2]["attributes"]["query"], "Search target");
  assert_eq!(items[2]["attributes"]["maxResults"], "12");
  assert_eq!(items[3]["kind"], "toolResult");
  assert_eq!(items[3]["attributes"]["resultCount"], "2");
  assert_eq!(items[3]["attributes"]["uniquePathCount"], "2");
  assert_eq!(items[3]["attributes"]["maxResults"], "12");
  assert!(items[3]["attributes"].get("nextAction").is_none());
  assert!(items[3]["content"]
    .as_str()
    .unwrap()
    .contains("README.md:2"));
  assert!(items[3]["content"]
    .as_str()
    .unwrap()
    .contains("docs/notes.txt:1"));
  assert_eq!(items[4]["kind"], "assistantMessage");
}

#[test]
fn turn_start_reads_single_file_search_result_as_second_step() {
  let mut context = RuntimeContext::new_in_memory();
  enable_full_access_plugin(&mut context);
  let workspace = create_temp_workspace("search-then-read");
  fs::write(
    workspace.join("README.md"),
    "# Pith\nUnique target lives here\nSecond line\n",
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
        "title": "Search Then Read Thread"
      })),
    ),
  );

  let turn_response = handle_request(
    &mut context,
    request(
      methods::TURN_START,
      Some(json!({
        "threadId": "thread-1",
        "message": "Find Unique target"
      })),
    ),
  );

  fs::remove_dir_all(&workspace).expect("cleanup temp workspace");

  assert!(turn_response.error.is_none());
  let result = turn_response.result.expect("turn result");
  let items = result["items"].as_array().expect("items");

  assert_eq!(items[2]["kind"], "toolStart");
  assert_eq!(items[2]["title"], "search_files");
  assert_eq!(items[3]["kind"], "toolResult");
  assert_eq!(items[3]["attributes"]["resultCount"], "1");
  assert_eq!(items[3]["attributes"]["uniquePathCount"], "1");
  assert_eq!(items[3]["attributes"]["nextAction"], "read_file");
  assert_eq!(items[3]["attributes"]["nextRelativePath"], "README.md");
  assert_eq!(items[3]["attributes"]["agentStepIndex"], "1");
  assert_eq!(items[3]["attributes"]["agentLoopStepCount"], "1");
  assert_eq!(items[3]["attributes"]["agentLoopStopReason"], "completed");
  assert_eq!(items[5]["kind"], "toolStart");
  assert_eq!(items[5]["title"], "read_file");
  assert_eq!(items[5]["attributes"]["relativePath"], "README.md");
  assert_eq!(items[5]["attributes"]["agentStepIndex"], "2");
  assert_eq!(items[5]["attributes"]["agentLoopStepCount"], "2");
  assert_eq!(items[5]["attributes"]["agentLoopBudgetRemaining"], "1");
  assert_eq!(items[6]["kind"], "toolResult");
  assert!(items[6]["content"]
    .as_str()
    .unwrap()
    .contains("Unique target lives here"));
  assert_eq!(items[7]["kind"], "assistantMessage");
  assert_eq!(items[7]["attributes"]["agentStepIndex"], "2");
  assert_eq!(items[7]["attributes"]["agentLoopStopReason"], "streaming");
  assert_eq!(result["activeTurnId"].as_str().unwrap(), "thread-1-turn-1");
}
