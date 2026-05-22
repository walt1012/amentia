#![cfg(unix)]

use super::test_support::{
  bundled_manifest_plugin_entry, create_temp_workspace, replace_plugin_catalog, request,
};
use super::*;
use pith_protocol::methods;
use serde_json::json;
use std::fs;

#[test]
fn bundled_notion_connector_runs_local_mcp_draft_after_approval() {
  let mut context = RuntimeContext::new_in_memory();
  let workspace = create_temp_workspace("bundled-notion-local-draft");
  replace_plugin_catalog(
    &mut context,
    vec![bundled_manifest_plugin_entry(
      "notion-connector",
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
        "title": "Bundled Notion Connector Thread"
      })),
    ),
  );
  let authorize_response = handle_request(
    &mut context,
    request(
      methods::PLUGIN_CONNECTOR_AUTHORIZE,
      Some(json!({
        "connectorId": "notion-connector::notion",
        "credentialSecret": "notion-local-token"
      })),
    ),
  );
  assert!(authorize_response.error.is_none());

  let response = handle_request(
    &mut context,
    request(
      methods::PLUGIN_COMMAND_RUN,
      Some(json!({
        "threadId": "thread-1",
        "commandId": "notion-connector::notion.prepare-page-draft",
        "input": "Prepare a project handoff page"
      })),
    ),
  );

  assert!(response.error.is_none());
  let result = response.result.expect("command run result");
  let items = result["items"].as_array().expect("items");
  assert_eq!(items[0]["kind"], "pluginCommand");
  assert_eq!(items[1]["kind"], "approvalRequested");
  assert_eq!(
    items[1]["attributes"]["connectorId"],
    "notion-connector::notion"
  );
  assert_eq!(items[1]["attributes"]["connectorServices"], "notion");
  assert_eq!(
    items[1]["attributes"]["executionKind"],
    "mcp.notion.preparePageDraft"
  );
  assert_eq!(
    result["pendingApprovals"][0]["title"],
    "Run Prepare Notion Page Draft"
  );
  let approval_id = result["pendingApprovals"][0]["id"]
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
  assert!(!draft_item.to_string().contains("notion-local-token"));

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
