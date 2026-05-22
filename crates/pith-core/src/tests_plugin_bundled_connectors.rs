#![cfg(unix)]

use super::test_support::{
  bundled_manifest_plugin_entry, create_temp_workspace, replace_plugin_catalog, request,
};
use super::*;
use pith_protocol::methods;
use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;

const NOTION_PLUGIN_ID: &str = "notion-connector";
const NOTION_CONNECTOR_ID: &str = "notion-connector::notion";
const NOTION_COMMAND_ID: &str = "notion-connector::notion.prepare-page-draft";
const NOTION_SECRET: &str = "notion-local-token";

#[test]
fn bundled_notion_connector_runs_local_mcp_draft_after_approval() {
  let (mut context, workspace) = setup_authorized_notion_context(
    "bundled-notion-local-draft",
    "Bundled Notion Connector Thread",
  );

  let response = handle_request(
    &mut context,
    request(
      methods::PLUGIN_COMMAND_RUN,
      Some(json!({
        "threadId": "thread-1",
        "commandId": NOTION_COMMAND_ID,
        "input": "Prepare a project handoff page"
      })),
    ),
  );

  assert!(response.error.is_none());
  let result = response.result.expect("command run result");
  let items = result["items"].as_array().expect("items");
  assert_eq!(items[0]["kind"], "pluginCommand");
  assert_eq!(items[1]["kind"], "approvalRequested");
  assert_eq!(items[1]["attributes"]["connectorId"], NOTION_CONNECTOR_ID);
  assert_eq!(items[1]["attributes"]["connectorServices"], "notion");
  assert_eq!(
    items[1]["attributes"]["executionKind"],
    "mcp.notion.preparePageDraft"
  );
  assert_eq!(
    result["pendingApprovals"][0]["title"],
    "Run Prepare Notion Page Draft"
  );

  let approval_result = approve_pending(&mut context, &result);
  fs::remove_dir_all(&workspace).expect("cleanup temp workspace");

  let items = approval_result["items"].as_array().expect("approval items");
  assert_local_draft_items(&context, items);
}

#[test]
fn bundled_notion_connector_turn_resumes_the_same_agent_step() {
  let (mut context, workspace) =
    setup_authorized_notion_context("bundled-notion-turn-loop", "Bundled Notion Turn Thread");

  let response = handle_request(
    &mut context,
    request(
      methods::TURN_START,
      Some(json!({
        "threadId": "thread-1",
        "message": format!("/plugin {NOTION_COMMAND_ID} Prepare a project handoff page")
      })),
    ),
  );

  assert!(response.error.is_none());
  let result = response.result.expect("turn result");
  let items = result["items"].as_array().expect("turn items");
  assert_eq!(items[0]["kind"], "userMessage");
  assert_eq!(items[1]["kind"], "pluginCommand");
  assert_eq!(items[1]["attributes"]["agentToolKind"], "connector");
  assert_eq!(items[1]["attributes"]["agentStepIndex"], "1");
  assert_eq!(items[2]["kind"], "approvalRequested");
  assert_eq!(
    items[2]["attributes"]["agentLoopStopReason"],
    "approvalPaused"
  );
  assert_eq!(items[2]["attributes"]["connectorId"], NOTION_CONNECTOR_ID);

  let approval_result = approve_pending(&mut context, &result);
  fs::remove_dir_all(&workspace).expect("cleanup temp workspace");

  let approved_items = approval_result["items"].as_array().expect("approval items");
  let draft_item = approved_items
    .iter()
    .find(|item| item["title"] == "Notion Page Draft")
    .expect("notion draft item");
  assert_eq!(draft_item["attributes"]["agentStepResume"], "true");
  assert_eq!(draft_item["attributes"]["agentToolKind"], "connector");
  assert_eq!(draft_item["attributes"]["agentStepIndex"], "1");
  assert_eq!(draft_item["attributes"]["agentLoopStopReason"], "completed");
  assert_eq!(draft_item["attributes"]["agentLoopObservationCount"], "1");
  assert_local_draft_items(&context, approved_items);
}

fn setup_authorized_notion_context(label: &str, title: &str) -> (RuntimeContext, PathBuf) {
  let mut context = RuntimeContext::new_in_memory();
  let workspace = create_temp_workspace(label);
  replace_plugin_catalog(
    &mut context,
    vec![bundled_manifest_plugin_entry(
      NOTION_PLUGIN_ID,
      "Notion Connector",
      true,
      false,
      &[
        "command:notion.prepare-page-draft",
        "connector:notion",
        "mcp_server:notion",
        "prompt_pack:notion.workspace",
      ],
      &["network.outbound", "mcp.connect"],
    )],
  );

  let workspace_response = handle_request(
    &mut context,
    request(
      methods::WORKSPACE_OPEN,
      Some(json!({
        "path": workspace.display().to_string()
      })),
    ),
  );
  assert!(workspace_response.error.is_none());
  let thread_response = handle_request(
    &mut context,
    request(
      methods::THREAD_START,
      Some(json!({
        "title": title
      })),
    ),
  );
  assert!(thread_response.error.is_none());
  let authorize_response = handle_request(
    &mut context,
    request(
      methods::PLUGIN_CONNECTOR_AUTHORIZE,
      Some(json!({
        "connectorId": NOTION_CONNECTOR_ID,
        "credentialSecret": NOTION_SECRET
      })),
    ),
  );
  assert!(authorize_response.error.is_none());

  (context, workspace)
}

fn approve_pending(context: &mut RuntimeContext, result: &Value) -> Value {
  let approval_id = result["pendingApprovals"][0]["id"]
    .as_str()
    .expect("approval id")
    .to_string();
  let approval_response = handle_request(
    context,
    request(
      methods::APPROVAL_RESPOND,
      Some(json!({
        "approvalId": approval_id,
        "decision": "approved"
      })),
    ),
  );
  assert!(approval_response.error.is_none());
  approval_response.result.expect("approval result")
}

fn assert_local_draft_items(context: &RuntimeContext, items: &[Value]) {
  let draft_item = items
    .iter()
    .find(|item| item["title"] == "Notion Page Draft")
    .expect("notion draft item");
  assert_eq!(draft_item["kind"], "pluginResult");
  assert_eq!(draft_item["attributes"]["targetService"], "notion");
  assert_eq!(draft_item["attributes"]["draftMode"], "localDraft");
  assert_eq!(draft_item["attributes"]["remoteWrite"], "false");
  assert_eq!(draft_item["attributes"]["credentialScoped"], "true");
  assert_eq!(
    draft_item["attributes"]["targetTool"],
    "notion.preparePageDraft"
  );
  assert_eq!(
    draft_item["attributes"]["pluginRunnerExecutionDriver"],
    "mcp"
  );
  assert_eq!(
    draft_item["attributes"]["pluginRunnerEntrypoint"],
    "notion.preparePageDraft"
  );
  assert_eq!(draft_item["attributes"]["mcpServerId"], "notion");
  assert_eq!(draft_item["attributes"]["mcpToolName"], "preparePageDraft");
  assert_eq!(
    draft_item["attributes"]["mcpServerCommand"],
    "bin/notion-mcp-local-draft.sh"
  );
  assert_eq!(draft_item["attributes"]["mcpProtocolStatus"], "completed");
  assert_eq!(
    draft_item["attributes"]["mcpStructuredContentStatus"],
    "pithOutputEnvelope"
  );
  assert_eq!(
    draft_item["attributes"]["pluginRunnerSecretBindings"],
    "env-bound"
  );
  assert!(draft_item["content"]
    .as_str()
    .expect("draft content")
    .contains("local Notion page draft"));
  assert!(!draft_item.to_string().contains(NOTION_SECRET));

  let memory_item = items
    .iter()
    .find(|item| item["title"] == "Plugin Memory Note Saved")
    .expect("notion memory item");
  assert_eq!(
    memory_item["attributes"]["memoryNoteTitle"],
    "Notion Draft Prepared"
  );
  let saved_note = context
    .memory_state
    .recent_notes(16)
    .into_iter()
    .find(|note| note.title == "Notion Draft Prepared")
    .expect("saved notion draft memory note");
  assert_eq!(saved_note.source, "plugin.notion-connector");
  assert!(saved_note
    .body
    .contains("credential-scoped local page draft"));
}
