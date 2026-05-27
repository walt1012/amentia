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
const NOTION_WRITE_INSPECTION_COMMAND_ID: &str = "notion-connector::notion.inspect-page-write";
const NOTION_SECRET: &str = "notion-local-token";
const SLACK_PLUGIN_ID: &str = "slack-connector";
const SLACK_CONNECTOR_ID: &str = "slack-connector::slack";
const SLACK_COMMAND_ID: &str = "slack-connector::slack.prepare-message-draft";
const SLACK_SECRET: &str = "slack-local-token";

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
  assert_connector_handoff_items(items);
}

#[test]
fn bundled_notion_connector_natural_turn_resumes_the_same_agent_step() {
  let (mut context, workspace) =
    setup_authorized_notion_context("bundled-notion-turn-loop", "Bundled Notion Turn Thread");

  let response = handle_request(
    &mut context,
    request(
      methods::TURN_START,
      Some(json!({
        "threadId": "thread-1",
        "message": "Prepare a Notion page draft for this project handoff."
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
  let handoff_item = assert_connector_handoff_items(approved_items);
  assert_eq!(handoff_item["attributes"]["agentStepResume"], "true");
  assert_eq!(handoff_item["attributes"]["agentStepPhase"], "final");
  assert_eq!(handoff_item["attributes"]["agentStepIndex"], "1");
  assert_eq!(handoff_item["attributes"]["agentToolKind"], "connector");
  assert_eq!(
    handoff_item["attributes"]["agentLoopStopReason"],
    "completed"
  );
  assert_eq!(handoff_item["attributes"]["agentLoopObservationCount"], "1");
}

#[test]
fn bundled_notion_connector_natural_turn_carries_saved_handoff_reference() {
  let (mut context, workspace) = setup_authorized_notion_context(
    "bundled-notion-saved-handoff",
    "Bundled Notion Saved Handoff Thread",
  );
  fs::create_dir_all(workspace.join("docs")).expect("create docs directory");
  fs::write(
    workspace.join("docs").join("handoff.md"),
    "# Project Handoff\n\nShip the practical cowork connector path.",
  )
  .expect("write saved handoff");

  let response = handle_request(
    &mut context,
    request(
      methods::TURN_START,
      Some(json!({
        "threadId": "thread-1",
        "message": "Prepare a Notion update from docs/handoff.md."
      })),
    ),
  );

  assert!(response.error.is_none());
  let result = response.result.expect("turn result");
  let items = result["items"].as_array().expect("turn items");
  assert_eq!(items[1]["kind"], "pluginCommand");
  assert_eq!(items[1]["attributes"]["agentToolKind"], "connector");
  assert_eq!(items[1]["attributes"]["commandId"], NOTION_COMMAND_ID);
  assert_eq!(
    items[1]["attributes"]["commandInput"],
    "Prepare a Notion update from docs/handoff.md.\n\nSaved artifact: docs/handoff.md\nSaved artifact preview: # Project Handoff Ship the practical cowork connector path.\nSaved artifact truncated: false"
  );
  assert_eq!(items[2]["attributes"]["connectorId"], NOTION_CONNECTOR_ID);
  assert_eq!(
    items[2]["attributes"]["commandInput"],
    "Prepare a Notion update from docs/handoff.md.\n\nSaved artifact: docs/handoff.md\nSaved artifact preview: # Project Handoff Ship the practical cowork connector path.\nSaved artifact truncated: false"
  );

  let approval_result = approve_pending(&mut context, &result);
  fs::remove_dir_all(&workspace).expect("cleanup temp workspace");

  let approved_items = approval_result["items"].as_array().expect("approval items");
  let draft_item = approved_items
    .iter()
    .find(|item| item["title"] == "Notion Page Draft")
    .expect("notion draft item");
  assert_eq!(
    draft_item["attributes"]["sourceArtifact"],
    "docs/handoff.md"
  );
  assert_eq!(
    draft_item["attributes"]["sourceArtifactPreviewProvided"],
    "true"
  );
  assert_eq!(draft_item["attributes"]["sourceArtifactTruncated"], "false");
  assert!(draft_item["content"]
    .as_str()
    .expect("draft content")
    .contains("docs/handoff.md"));
  assert!(draft_item["content"]
    .as_str()
    .expect("draft content")
    .contains("Ship the practical cowork connector path."));
  assert_local_draft_items(&context, approved_items);
  assert_connector_handoff_items(approved_items);

  let saved_note = context
    .memory_state
    .recent_notes(16)
    .into_iter()
    .find(|note| note.title == "Notion Draft Prepared")
    .expect("saved notion draft memory note");
  assert!(saved_note.body.contains("docs/handoff.md"));
  assert!(saved_note
    .body
    .contains("Ship the practical cowork connector path."));
}

#[test]
fn bundled_notion_connector_natural_publish_request_routes_to_write_inspection() {
  let (mut context, workspace) = setup_authorized_notion_context(
    "bundled-notion-natural-write-inspection",
    "Bundled Notion Natural Write Inspection Thread",
  );
  fs::create_dir_all(workspace.join("docs")).expect("create docs directory");
  fs::write(
    workspace.join("docs").join("handoff.md"),
    "# Project Handoff\n\nPrepare a safe Notion update.",
  )
  .expect("write saved handoff");

  let response = handle_request(
    &mut context,
    request(
      methods::TURN_START,
      Some(json!({
        "threadId": "thread-1",
        "message": "Publish a Notion update from docs/handoff.md."
      })),
    ),
  );

  assert!(response.error.is_none());
  let result = response.result.expect("turn result");
  let items = result["items"].as_array().expect("turn items");
  assert_eq!(items[1]["kind"], "pluginCommand");
  assert_eq!(
    items[1]["attributes"]["commandId"],
    NOTION_WRITE_INSPECTION_COMMAND_ID
  );
  assert_eq!(
    items[1]["attributes"]["toolPlanningSelectedCommandId"],
    NOTION_WRITE_INSPECTION_COMMAND_ID
  );
  assert_eq!(items[2]["attributes"]["connectorId"], NOTION_CONNECTOR_ID);

  let approval_result = approve_pending(&mut context, &result);
  fs::remove_dir_all(&workspace).expect("cleanup temp workspace");

  let approved_items = approval_result["items"].as_array().expect("approval items");
  assert!(approved_items
    .iter()
    .any(|item| item["title"] == "Notion Remote Write Inspection"));
}

#[test]
fn bundled_notion_connector_inspects_remote_write_after_approval() {
  let (mut context, workspace) = setup_authorized_notion_context(
    "bundled-notion-write-inspection",
    "Bundled Notion Write Inspection Thread",
  );
  fs::create_dir_all(workspace.join("docs")).expect("create docs directory");
  fs::write(
    workspace.join("docs").join("handoff.md"),
    "# Project Handoff\n\nPrepare a safe Notion update.",
  )
  .expect("write saved handoff");

  let response = handle_request(
    &mut context,
    request(
      methods::PLUGIN_COMMAND_RUN,
      Some(json!({
        "threadId": "thread-1",
        "commandId": NOTION_WRITE_INSPECTION_COMMAND_ID,
        "input": "Publish a Notion update from docs/handoff.md.\n\nSaved artifact: docs/handoff.md\nSaved artifact preview: # Project Handoff Prepare a safe Notion update.\nSaved artifact truncated: false"
      })),
    ),
  );

  assert!(response.error.is_none());
  let result = response.result.expect("command run result");
  let items = result["items"].as_array().expect("items");
  assert_eq!(items[0]["kind"], "pluginCommand");
  assert_eq!(items[1]["kind"], "approvalRequested");
  assert_eq!(
    items[1]["attributes"]["commandId"],
    NOTION_WRITE_INSPECTION_COMMAND_ID
  );
  assert_eq!(
    items[1]["attributes"]["executionKind"],
    "mcp.notion.inspectPageWrite"
  );
  assert_eq!(
    result["pendingApprovals"][0]["title"],
    "Run Inspect Notion Page Write"
  );

  let approval_result = approve_pending(&mut context, &result);
  fs::remove_dir_all(&workspace).expect("cleanup temp workspace");

  let approved_items = approval_result["items"].as_array().expect("approval items");
  let inspection_item = approved_items
    .iter()
    .find(|item| item["title"] == "Notion Remote Write Inspection")
    .expect("notion write inspection item");
  assert_eq!(inspection_item["kind"], "pluginResult");
  assert_eq!(inspection_item["attributes"]["targetService"], "notion");
  assert_eq!(
    inspection_item["attributes"]["draftMode"],
    "remoteWriteInspection"
  );
  assert_eq!(inspection_item["attributes"]["remoteWrite"], "false");
  assert_eq!(
    inspection_item["attributes"]["remoteWriteStage"],
    "inspectBeforeWrite"
  );
  assert_eq!(
    inspection_item["attributes"]["remoteWriteRequiresApproval"],
    "true"
  );
  assert_eq!(
    inspection_item["attributes"]["remoteWriteStatus"],
    "notSent"
  );
  assert_eq!(
    inspection_item["attributes"]["targetTool"],
    "notion.inspectPageWrite"
  );
  assert_eq!(
    inspection_item["attributes"]["sourceArtifact"],
    "docs/handoff.md"
  );
  assert_eq!(
    inspection_item["attributes"]["connectorWorkflowId"],
    "notion.create-page"
  );
  assert_eq!(
    inspection_item["attributes"]["connectorWorkflowStatus"],
    "inspected"
  );
  assert!(inspection_item["content"]
    .as_str()
    .expect("inspection content")
    .contains("No remote write was sent"));
  assert!(!inspection_item.to_string().contains(NOTION_SECRET));
  let handoff_item = assert_inspection_handoff_items(approved_items);
  assert_eq!(
    handoff_item["attributes"]["remoteWriteStage"],
    "inspectBeforeWrite"
  );
  assert_eq!(
    handoff_item["attributes"]["remoteWriteRequiresApproval"],
    "true"
  );
  assert_eq!(handoff_item["attributes"]["remoteWriteStatus"], "notSent");
  assert_eq!(
    handoff_item["attributes"]["sourceArtifact"],
    "docs/handoff.md"
  );
  assert_eq!(
    handoff_item["attributes"]["connectorWorkflowId"],
    "notion.create-page"
  );
  assert_eq!(
    handoff_item["attributes"]["connectorWorkflowStatus"],
    "inspected"
  );

  let saved_note = context
    .memory_state
    .recent_notes(16)
    .into_iter()
    .find(|note| note.title == "Notion Write Inspected")
    .expect("saved notion inspection memory note");
  assert!(saved_note.body.contains("docs/handoff.md"));
}

#[test]
fn bundled_slack_connector_runs_local_mcp_draft_after_approval() {
  let (mut context, workspace) = setup_authorized_slack_context(
    "bundled-slack-local-draft",
    "Bundled Slack Connector Thread",
  );

  let response = handle_request(
    &mut context,
    request(
      methods::PLUGIN_COMMAND_RUN,
      Some(json!({
        "threadId": "thread-1",
        "commandId": SLACK_COMMAND_ID,
        "input": "Prepare a Slack update for the cowork handoff"
      })),
    ),
  );

  assert!(response.error.is_none());
  let result = response.result.expect("command run result");
  let items = result["items"].as_array().expect("items");
  assert_eq!(items[0]["kind"], "pluginCommand");
  assert_eq!(items[1]["kind"], "approvalRequested");
  assert_eq!(items[1]["attributes"]["connectorId"], SLACK_CONNECTOR_ID);
  assert_eq!(items[1]["attributes"]["connectorServices"], "slack");
  assert_eq!(
    items[1]["attributes"]["executionKind"],
    "mcp.slack.prepareMessageDraft"
  );
  assert_eq!(
    result["pendingApprovals"][0]["title"],
    "Run Prepare Slack Message Draft"
  );

  let approval_result = approve_pending(&mut context, &result);
  fs::remove_dir_all(&workspace).expect("cleanup temp workspace");

  let items = approval_result["items"].as_array().expect("approval items");
  assert_local_slack_draft_items(&context, items);
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
        "command:notion.inspect-page-write",
        "command:notion.publish-page-draft",
        "connector:notion",
        "connector_workflow:notion.create-page",
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

fn setup_authorized_slack_context(label: &str, title: &str) -> (RuntimeContext, PathBuf) {
  let mut context = RuntimeContext::new_in_memory();
  let workspace = create_temp_workspace(label);
  replace_plugin_catalog(
    &mut context,
    vec![bundled_manifest_plugin_entry(
      SLACK_PLUGIN_ID,
      "Slack Connector",
      true,
      false,
      &[
        "command:slack.prepare-message-draft",
        "command:slack.inspect-message-send",
        "command:slack.post-message-draft",
        "connector:slack",
        "connector_workflow:slack.post-message",
        "mcp_server:slack",
        "prompt_pack:slack.workspace",
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
        "connectorId": SLACK_CONNECTOR_ID,
        "credentialSecret": SLACK_SECRET
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

fn assert_local_slack_draft_items(context: &RuntimeContext, items: &[Value]) {
  let draft_item = items
    .iter()
    .find(|item| item["title"] == "Slack Message Draft")
    .expect("slack draft item");
  assert_eq!(draft_item["kind"], "pluginResult");
  assert_eq!(draft_item["attributes"]["targetService"], "slack");
  assert_eq!(draft_item["attributes"]["draftMode"], "localDraft");
  assert_eq!(draft_item["attributes"]["remoteWrite"], "false");
  assert_eq!(draft_item["attributes"]["credentialScoped"], "true");
  assert_eq!(
    draft_item["attributes"]["targetTool"],
    "slack.prepareMessageDraft"
  );
  assert_eq!(
    draft_item["attributes"]["connectorWorkflowId"],
    "slack.post-message"
  );
  assert_eq!(
    draft_item["attributes"]["connectorWorkflowStatus"],
    "prepared"
  );
  assert_eq!(
    draft_item["attributes"]["pluginRunnerExecutionDriver"],
    "mcp"
  );
  assert_eq!(draft_item["attributes"]["mcpServerId"], "slack");
  assert_eq!(draft_item["attributes"]["mcpToolName"], "prepareMessageDraft");
  assert_eq!(
    draft_item["attributes"]["mcpServerCommand"],
    "bin/slack-mcp-local-message.sh"
  );
  assert_eq!(draft_item["attributes"]["mcpProtocolStatus"], "completed");
  assert_eq!(
    draft_item["attributes"]["pluginRunnerSecretBindings"],
    "env-bound"
  );
  assert_eq!(
    draft_item["attributes"]["nextCommandId"],
    "slack-connector::slack.post-message-draft"
  );
  assert!(draft_item["content"]
    .as_str()
    .expect("draft content")
    .contains("local Slack message draft"));
  assert!(!draft_item.to_string().contains(SLACK_SECRET));

  let memory_item = items
    .iter()
    .find(|item| item["title"] == "Plugin Memory Note Saved")
    .expect("slack memory item");
  assert_eq!(
    memory_item["attributes"]["memoryNoteTitle"],
    "Slack Draft Prepared"
  );
  let saved_note = context
    .memory_state
    .recent_notes(16)
    .into_iter()
    .find(|note| note.title == "Slack Draft Prepared")
    .expect("saved slack draft memory note");
  assert_eq!(saved_note.source, "plugin.slack-connector");
  assert!(saved_note.body.contains("Slack connector prepared"));
}

fn assert_connector_handoff_items(items: &[Value]) -> &Value {
  let handoff_item = items
    .iter()
    .find(|item| {
      item["kind"] == "assistantMessage"
        && item["attributes"]["pluginCommandHandoff"] == "approvedPluginCommand"
    })
    .expect("connector handoff item");
  assert_eq!(handoff_item["attributes"]["pluginId"], NOTION_PLUGIN_ID);
  assert_eq!(handoff_item["attributes"]["commandId"], NOTION_COMMAND_ID);
  assert_eq!(handoff_item["attributes"]["connectorServices"], "notion");
  assert_eq!(
    handoff_item["attributes"]["pluginCommandObservationTitle"],
    "Notion Page Draft"
  );
  assert!(handoff_item["content"]
    .as_str()
    .expect("handoff content")
    .contains("Prepare Notion Page Draft completed"));
  assert!(handoff_item["content"]
    .as_str()
    .expect("handoff content")
    .contains("local Notion page draft"));
  assert!(!handoff_item.to_string().contains(NOTION_SECRET));
  handoff_item
}

fn assert_inspection_handoff_items(items: &[Value]) -> &Value {
  let handoff_item = items
    .iter()
    .find(|item| {
      item["kind"] == "assistantMessage"
        && item["attributes"]["pluginCommandHandoff"] == "approvedPluginCommand"
        && item["attributes"]["commandId"] == NOTION_WRITE_INSPECTION_COMMAND_ID
    })
    .expect("inspection handoff item");
  assert_eq!(handoff_item["attributes"]["pluginId"], NOTION_PLUGIN_ID);
  assert_eq!(
    handoff_item["attributes"]["pluginCommandObservationTitle"],
    "Notion Remote Write Inspection"
  );
  assert_eq!(handoff_item["attributes"]["targetService"], "notion");
  assert_eq!(
    handoff_item["attributes"]["targetTool"],
    "notion.inspectPageWrite"
  );
  assert_eq!(handoff_item["attributes"]["remoteWrite"], "false");
  assert!(handoff_item["content"]
    .as_str()
    .expect("handoff content")
    .contains("No remote write was sent"));
  assert!(!handoff_item.to_string().contains(NOTION_SECRET));
  handoff_item
}
