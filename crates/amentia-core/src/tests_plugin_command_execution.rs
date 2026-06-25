use super::test_support::{
  bundled_manifest_plugin_entry, create_temp_plugin_bundle, create_temp_workspace,
  make_test_file_executable, remove_temp_plugin_source, remove_temp_workspace,
  replace_plugin_catalog, request, temp_manifest_plugin_entry,
};
use super::*;
use amentia_plugin_host::PluginCatalogEntry;
use amentia_protocol::methods;
use amentia_storage::RuntimeStore;
use serde_json::json;
use std::fs;

#[cfg(unix)]
fn create_notion_stdio_runner_plugin(label: &str) -> (std::path::PathBuf, PluginCatalogEntry) {
  let source_root = create_temp_plugin_bundle(label, "notion-runner", "Notion Runner");
  let plugin_manifest = source_root.join("amentia-plugin.json");
  let runner_path = source_root.join("runner.sh");
  fs::write(
    &plugin_manifest,
    r#"{
  "name": "notion-runner",
  "version": "0.1.0",
  "displayName": "Notion Runner",
  "description": "Connector-backed stdio command plugin",
  "author": { "name": "Amentia" },
  "capabilities": ["command:notion-runner.sync", "connector:notion"],
  "permissions": ["network.outbound", "mcp.connect"],
  "appConnectors": [
    {
      "id": "notion",
      "displayName": "Notion",
      "service": "notion",
      "homepage": "https://www.notion.so"
    }
  ],
  "authPolicy": {
    "type": "oauth2",
    "required": true,
    "scopes": ["read_content"],
    "credentialStore": "local"
  },
  "defaultEnabled": true
}"#,
  )
  .expect("write connector runner plugin manifest");
  fs::write(
    source_root.join("commands").join("notion-runner.sync.json"),
    r#"{
  "title": "Sync Notion",
  "description": "Run a connector-backed stdio command.",
  "prompt": "Sync local context with Notion.",
  "execution": {
    "kind": "stdio.notionSync",
    "entrypoint": "runner.sh"
  }
}"#,
  )
  .expect("write connector command manifest");
  fs::write(
    &runner_path,
    r#"#!/bin/sh
payload=$(cat)
case "$payload" in *'"connectorId":"notion-runner::notion"'*) connector_id=true;; *) connector_id=false;; esac
case "$payload" in *'"provider":"amentia.localCredentialProvider"'*) provider=true;; *) provider=false;; esac
case "$payload" in *'"handle":"notion-runner::notion"'*) handle=true;; *) handle=false;; esac
case "$payload" in *'"store":"local"'*) store=true;; *) store=false;; esac
case "$payload" in *'"label":"Notion authorization marker"'*) label=true;; *) label=false;; esac
case "$payload" in *"access_token"*|*"refresh_token"*|*"secret"*) secret_leak=true;; *) secret_leak=false;; esac
printf '{"content":"connectorId=%s provider=%s handle=%s store=%s label=%s secretLeak=%s","memoryNotes":[{"title":"Approved Connector Memory","body":"Connector runner memory survives approval execution.","source":"plugin.notion-runner.approved","tags":["connector","approved"]}]}\n' "$connector_id" "$provider" "$handle" "$store" "$label" "$secret_leak"
"#,
  )
  .expect("write connector runner");
  make_test_file_executable(&runner_path, "connector runner");
  (
    source_root,
    temp_manifest_plugin_entry(
      "notion-runner",
      "Notion Runner",
      "Connector-backed stdio command plugin",
      &["command:notion-runner.sync", "connector:notion"],
      &["network.outbound", "mcp.connect"],
      &plugin_manifest,
    ),
  )
}

#[cfg(unix)]
fn create_next_action_stdio_runner_plugin(
  label: &str,
  plugin_name: &str,
  output_json: &str,
  permissions: &[&str],
) -> (std::path::PathBuf, PluginCatalogEntry) {
  let source_root = create_temp_plugin_bundle(label, plugin_name, "Follow-up Runner");
  let plugin_manifest = source_root.join("amentia-plugin.json");
  let runner_path = source_root.join("runner.sh");
  let command_name = format!("{plugin_name}.run");
  fs::write(
    &plugin_manifest,
    format!(
      r#"{{
  "name": "{plugin_name}",
  "version": "0.1.0",
  "displayName": "Follow-up Runner",
  "description": "Emits next-action observations for the agent loop.",
  "author": {{ "name": "Amentia" }},
  "capabilities": ["command:{command_name}"],
  "permissions": [{}],
  "defaultEnabled": true
}}"#,
      permissions
        .iter()
        .map(|permission| format!(r#""{permission}""#))
        .collect::<Vec<_>>()
        .join(", ")
    ),
  )
  .expect("write follow-up plugin manifest");
  fs::write(
    source_root
      .join("commands")
      .join(format!("{command_name}.json")),
    r#"{
  "title": "Emit Follow-up",
  "description": "Emit a next action observation.",
  "prompt": "Emit a next action for the agent loop.",
  "execution": {
    "kind": "stdio.followUp",
    "entrypoint": "runner.sh"
  }
}"#,
  )
  .expect("write follow-up command manifest");
  fs::write(
    &runner_path,
    format!(
      r#"#!/bin/sh
cat >/dev/null
cat <<'JSON'
{output_json}
JSON
"#
    ),
  )
  .expect("write follow-up runner");
  make_test_file_executable(&runner_path, "follow-up runner");
  let capability = format!("command:{command_name}");
  (
    source_root,
    temp_manifest_plugin_entry(
      plugin_name,
      "Follow-up Runner",
      "Emits next-action observations for the agent loop.",
      &[capability.as_str()],
      permissions,
      &plugin_manifest,
    ),
  )
}

#[cfg(unix)]
fn create_linear_stdio_runner_plugin(label: &str) -> (std::path::PathBuf, PluginCatalogEntry) {
  let source_root = create_temp_plugin_bundle(label, "linear-runner", "Linear Runner");
  let plugin_manifest = source_root.join("amentia-plugin.json");
  let runner_path = source_root.join("runner.sh");
  fs::write(
    &plugin_manifest,
    r#"{
  "name": "linear-runner",
  "version": "0.1.0",
  "displayName": "Linear Runner",
  "description": "Connector-backed Linear command plugin",
  "author": { "name": "Amentia" },
  "capabilities": ["command:linear.update", "connector:linear"],
  "permissions": ["network.outbound"],
  "appConnectors": [
    {
      "id": "linear",
      "displayName": "Linear",
      "service": "linear",
      "homepage": "https://linear.app"
    }
  ],
  "defaultEnabled": true
}"#,
  )
  .expect("write linear runner plugin manifest");
  fs::write(
    source_root.join("commands").join("linear.update.json"),
    r#"{
  "title": "Prepare Linear Update",
  "description": "Prepare a Linear project update from the current cowork request.",
  "prompt": "Prepare a concise Linear project update.",
  "execution": {
    "kind": "stdio.linearUpdate",
    "entrypoint": "runner.sh",
    "connectors": ["linear"]
  }
}"#,
  )
  .expect("write linear command manifest");
  fs::write(
    &runner_path,
    r#"#!/bin/sh
payload=$(cat)
case "$payload" in *'"connectorId":"linear-runner::linear"'*) connector_id=true;; *) connector_id=false;; esac
case "$payload" in *'"service":"linear"'*) service=true;; *) service=false;; esac
case "$payload" in *"Prepare a Linear project update"*) input=true;; *) input=false;; esac
printf '{"content":"linear connector_id=%s service=%s input=%s","memoryNotes":[{"title":"Linear Update Draft","body":"Linear connector routing produced a project update draft.","source":"plugin.linear-runner","tags":["connector","linear"]}]}\n' "$connector_id" "$service" "$input"
"#,
  )
  .expect("write linear runner");
  make_test_file_executable(&runner_path, "linear runner");
  (
    source_root,
    temp_manifest_plugin_entry(
      "linear-runner",
      "Linear Runner",
      "Connector-backed Linear command plugin",
      &["command:linear.update", "connector:linear"],
      &["network.outbound"],
      &plugin_manifest,
    ),
  )
}

#[test]
fn plugin_command_run_executes_builtin_command_for_the_selected_thread() {
  let mut context = RuntimeContext::new_in_memory();
  let workspace = create_temp_workspace("plugin-command-run");
  replace_plugin_catalog(
    &mut context,
    vec![bundled_manifest_plugin_entry(
      "workspace-notes",
      "Workspace Notes",
      true,
      true,
      &["command:workspace.capture-note", "skill:workspace.notes"],
      &["file.read", "file.write"],
    )],
  );
  fs::write(
    workspace.join("README.md"),
    "Workspace A\nCommand registry path\n",
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
        "title": "Plugin Command Thread"
      })),
    ),
  );

  let response = handle_request(
    &mut context,
    request(
      methods::PLUGIN_COMMAND_RUN,
      Some(json!({
        "threadId": "thread-1",
        "commandId": "workspace-notes::workspace.capture-note"
      })),
    ),
  );

  remove_temp_workspace(&workspace);

  assert!(response.error.is_none());
  let result = response.result.expect("command run result");
  let items = result["items"].as_array().expect("items");
  assert_eq!(items[0]["kind"], "pluginCommand");
  assert_eq!(items[0]["attributes"]["pluginId"], "workspace-notes");
  assert_eq!(items[1]["kind"], "pluginResult");
  assert_eq!(
    items[1]["attributes"]["executionKind"],
    "builtin.workspaceReadmeNote"
  );
  assert!(items[1]["content"]
    .as_str()
    .unwrap()
    .contains("Command registry path"));
  assert_eq!(items[2]["kind"], "assistantMessage");
  let memory_item = items
    .iter()
    .find(|item| item["title"] == "Memory Note Saved")
    .expect("memory note saved item");
  assert_eq!(memory_item["kind"], "system");
  assert_eq!(memory_item["attributes"]["pluginId"], "workspace-notes");
  assert_eq!(result["threadId"], "thread-1");
  assert_eq!(context.memory_state.note_count(), 3);
  assert!(context
    .memory_state
    .recent_notes(16)
    .into_iter()
    .any(|note| note.title == "Workspace Capture" && note.source == "plugin.workspace-notes"));
}

#[test]
fn plugin_command_run_honors_pending_running_cancel_before_builtin_execution() {
  let mut context = RuntimeContext::new_in_memory();
  let workspace = create_temp_workspace("plugin-command-pending-cancel");
  replace_plugin_catalog(
    &mut context,
    vec![bundled_manifest_plugin_entry(
      "workspace-notes",
      "Workspace Notes",
      true,
      true,
      &["command:workspace.capture-note", "skill:workspace.notes"],
      &["file.read", "file.write"],
    )],
  );
  fs::write(workspace.join("README.md"), "Should not be captured\n").expect("write readme");

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
        "title": "Plugin Command Cancel Thread"
      })),
    ),
  );
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

  let response = handle_request(
    &mut context,
    request(
      methods::PLUGIN_COMMAND_RUN,
      Some(json!({
        "threadId": "thread-1",
        "commandId": "workspace-notes::workspace.capture-note",
        "input": "Cancel before execution"
      })),
    ),
  );

  remove_temp_workspace(&workspace);

  assert!(response.error.is_none());
  let result = response.result.expect("command run result");
  let items = result["items"].as_array().expect("items");
  assert_eq!(items.len(), 2);
  assert_eq!(items[0]["kind"], "pluginCommand");
  assert_eq!(items[1]["kind"], "warning");
  assert_eq!(items[1]["attributes"]["pluginCommandStatus"], "cancelled");
  assert_eq!(
    items[1]["attributes"]["pluginRunnerFailureKind"],
    "cancelled"
  );
  assert_eq!(
    items[1]["attributes"]["pluginRunnerRecoveryHint"],
    "Run the command again when the current task is ready."
  );
  assert_eq!(
    items[0]["attributes"]["pluginCommandRunId"],
    "thread-1::workspace-notes::workspace.capture-note"
  );
  assert_eq!(
    items[1]["attributes"]["pluginCommandRunId"],
    items[0]["attributes"]["pluginCommandRunId"]
  );
  assert_eq!(
    context
      .execution_state
      .counts()
      .running_plugin_command_count(),
    0
  );
  assert!(!context
    .memory_state
    .recent_notes(16)
    .into_iter()
    .any(|note| note.title == "Workspace Capture"));
}

#[test]
fn plugin_command_run_reports_repair_metadata_when_thread_is_missing() {
  let mut context = RuntimeContext::new_in_memory();
  replace_plugin_catalog(
    &mut context,
    vec![bundled_manifest_plugin_entry(
      "workspace-notes",
      "Workspace Notes",
      true,
      true,
      &["command:workspace.capture-note", "skill:workspace.notes"],
      &["file.read", "file.write"],
    )],
  );

  let response = handle_request(
    &mut context,
    request(
      methods::PLUGIN_COMMAND_RUN,
      Some(json!({
        "threadId": "missing-thread",
        "commandId": "workspace-notes::workspace.capture-note"
      })),
    ),
  );

  assert!(response.result.is_none());
  let error = response.error.expect("plugin command missing thread error");
  assert_eq!(error.code, -32004);
  let data = error.data.expect("plugin command missing thread data");
  assert_eq!(data["pluginId"], "workspace-notes");
  assert_eq!(data["commandId"], "workspace-notes::workspace.capture-note");
  assert_eq!(data["runStatus"], "missingThread");
  assert_eq!(data["runBlocker"], "Session not found");
  assert!(data["runRepairHint"]
    .as_str()
    .expect("run repair hint")
    .contains("Select or create a session"));
}

#[test]
fn plugin_command_run_reports_repair_metadata_when_command_is_missing() {
  let mut context = RuntimeContext::new_in_memory();

  let response = handle_request(
    &mut context,
    request(
      methods::PLUGIN_COMMAND_RUN,
      Some(json!({
        "threadId": "thread-1",
        "commandId": "missing-plugin::missing-command"
      })),
    ),
  );

  assert!(response.result.is_none());
  let error = response
    .error
    .expect("plugin command missing command error");
  assert_eq!(error.code, -32052);
  let data = error.data.expect("plugin command missing command data");
  assert_eq!(data["commandId"], "missing-plugin::missing-command");
  assert_eq!(data["runStatus"], "commandNotFound");
  assert_eq!(data["runBlocker"], "Plugin command not found");
  assert!(data["runRepairHint"]
    .as_str()
    .expect("run repair hint")
    .contains("Refresh plugins"));
}

#[test]
fn plugin_command_run_reports_repair_metadata_when_completion_persistence_fails() {
  let mut context = RuntimeContext::new_in_memory();
  let workspace = create_temp_workspace("plugin-command-completion-persist-fail");
  let storage_root = create_temp_workspace("plugin-command-completion-failing-storage");
  let database_path = storage_root.join("amentia.db");
  fs::create_dir_all(&database_path).expect("create directory at database path");
  replace_plugin_catalog(
    &mut context,
    vec![bundled_manifest_plugin_entry(
      "workspace-notes",
      "Workspace Notes",
      true,
      true,
      &["command:workspace.capture-note", "skill:workspace.notes"],
      &["file.read", "file.write"],
    )],
  );
  fs::write(workspace.join("README.md"), "Completion persistence test\n").expect("write readme");

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
        "title": "Plugin Command Persistence"
      })),
    ),
  );
  context
    .persistence_state
    .set_store_for_testing(RuntimeStore::new(database_path));

  let response = handle_request(
    &mut context,
    request(
      methods::PLUGIN_COMMAND_RUN,
      Some(json!({
        "threadId": "thread-1",
        "commandId": "workspace-notes::workspace.capture-note",
        "input": "persist this note"
      })),
    ),
  );

  remove_temp_workspace(&workspace);
  fs::remove_dir_all(&storage_root).expect("cleanup storage root");

  assert!(response.result.is_none());
  let error = response.error.expect("plugin command persistence error");
  assert_eq!(error.code, -32010);
  let data = error.data.expect("plugin command persistence error data");
  assert_eq!(data["pluginId"], "workspace-notes");
  assert_eq!(data["commandId"], "workspace-notes::workspace.capture-note");
  assert_eq!(data["commandInput"], "persist this note");
  assert_eq!(data["runStatus"], "persistFailed");
  assert!(data["runBlocker"].as_str().expect("run blocker").len() > 10);
  assert!(data["runRepairHint"]
    .as_str()
    .expect("run repair hint")
    .contains("storage permissions"));
}

#[test]
fn turn_start_routes_natural_workspace_note_through_plugin_execution() {
  let mut context = RuntimeContext::new_in_memory();
  let workspace = create_temp_workspace("turn-natural-note-route");
  replace_plugin_catalog(
    &mut context,
    vec![bundled_manifest_plugin_entry(
      "workspace-notes",
      "Workspace Notes",
      true,
      true,
      &["command:workspace.capture-note", "skill:workspace.notes"],
      &["file.read", "file.write"],
    )],
  );
  fs::write(
    workspace.join("README.md"),
    "Workspace Route\nNatural plugin routing\n",
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
        "title": "Turn Plugin Command Thread"
      })),
    ),
  );

  let response = handle_request(
    &mut context,
    request(
      methods::TURN_START,
      Some(json!({
        "threadId": "thread-1",
        "message": "Capture a workspace note for this project."
      })),
    ),
  );

  remove_temp_workspace(&workspace);

  assert!(response.error.is_none());
  let result = response.result.expect("turn result");
  let items = result["items"].as_array().expect("items");
  assert_eq!(items[0]["kind"], "userMessage");
  assert_eq!(items[1]["kind"], "pluginCommand");
  assert_eq!(
    items[1]["attributes"]["commandId"],
    "workspace-notes::workspace.capture-note"
  );
  assert_eq!(items[2]["kind"], "pluginResult");
  assert!(items[2]["content"]
    .as_str()
    .unwrap()
    .contains("Natural plugin routing"));
  assert_eq!(items[3]["kind"], "assistantMessage");
  assert_eq!(
    items[3]["attributes"]["pluginCommandHandoff"],
    "pluginCommand"
  );
  assert!(items
    .iter()
    .any(|item| item["title"] == "Memory Note Saved"));
  assert_eq!(result["activeTurnId"], serde_json::Value::Null);
  assert_eq!(context.memory_state.note_count(), 3);
}

#[test]
fn turn_start_routes_natural_review_diff_through_plugin_execution() {
  let mut context = RuntimeContext::new_in_memory();
  let workspace = create_temp_workspace("turn-natural-review-route");
  replace_plugin_catalog(
    &mut context,
    vec![bundled_manifest_plugin_entry(
      "review-assistant",
      "Review Assistant",
      true,
      true,
      &["command:review.inspect-diff"],
      &["file.read", "model.invoke"],
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
        "title": "Natural Review Thread"
      })),
    ),
  );

  let response = handle_request(
    &mut context,
    request(
      methods::TURN_START,
      Some(json!({
        "threadId": "thread-1",
        "message": "Review the current git diff."
      })),
    ),
  );

  remove_temp_workspace(&workspace);

  assert!(response.error.is_none());
  let result = response.result.expect("turn result");
  let items = result["items"].as_array().expect("items");
  assert_eq!(items[0]["kind"], "userMessage");
  assert_eq!(items[1]["kind"], "pluginCommand");
  assert_eq!(
    items[1]["attributes"]["commandId"],
    "review-assistant::review.inspect-diff"
  );
  assert_eq!(items[2]["kind"], "pluginResult");
  assert_eq!(items[3]["kind"], "assistantMessage");
  assert_eq!(
    items[3]["attributes"]["pluginCommandHandoff"],
    "pluginCommand"
  );
  assert_eq!(result["activeTurnId"], serde_json::Value::Null);
}

#[test]
fn turn_start_routes_review_summary_save_through_write_approval() {
  let mut context = RuntimeContext::new_in_memory();
  let workspace = create_temp_workspace("turn-review-save-route");
  replace_plugin_catalog(
    &mut context,
    vec![bundled_manifest_plugin_entry(
      "review-assistant",
      "Review Assistant",
      true,
      true,
      &["command:review.inspect-diff"],
      &["file.read", "file.write", "model.invoke"],
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
        "title": "Review Save Thread"
      })),
    ),
  );

  let turn_response = handle_request(
    &mut context,
    request(
      methods::TURN_START,
      Some(json!({
        "threadId": "thread-1",
        "message": "Review the current git diff and save a review summary."
      })),
    ),
  );

  assert!(turn_response.error.is_none());
  let turn_result = turn_response.result.expect("turn result");
  let items = turn_result["items"].as_array().expect("items");
  let review_result = items
    .iter()
    .find(|item| {
      item["kind"] == "pluginResult"
        && item["attributes"]["commandId"] == "review-assistant::review.inspect-diff"
    })
    .expect("review result");
  assert_eq!(review_result["attributes"]["nextAction"], "write_file");
  assert_eq!(
    review_result["attributes"]["nextRelativePath"],
    ".amentia/review-summary.md"
  );
  assert_eq!(
    review_result["attributes"]["reviewApplyMode"],
    "approvalRequiredWrite"
  );
  let diff = items
    .iter()
    .find(|item| item["kind"] == "diffArtifact")
    .expect("review summary diff");
  assert_eq!(
    diff["attributes"]["relativePath"],
    ".amentia/review-summary.md"
  );
  assert!(diff["content"]
    .as_str()
    .expect("diff content")
    .contains("Amentia Review Summary"));
  let approval = items
    .iter()
    .find(|item| item["kind"] == "approvalRequested")
    .expect("write approval");
  assert_eq!(approval["attributes"]["action"], "write_file");
  assert_eq!(
    approval["attributes"]["relativePath"],
    ".amentia/review-summary.md"
  );
  assert_eq!(
    approval["attributes"]["agentLoopStopReason"],
    "approvalPaused"
  );
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

  let written_summary = fs::read_to_string(workspace.join(".amentia").join("review-summary.md"))
    .expect("review summary");
  remove_temp_workspace(&workspace);

  assert!(approval_response.error.is_none());
  assert!(written_summary.contains("Amentia Review Summary"));
  assert!(written_summary.contains("Review the current git diff and save a review summary."));
}

#[cfg(unix)]
#[test]
fn turn_start_routes_natural_non_notion_connector_command() {
  let mut context = RuntimeContext::new_in_memory();
  let workspace = create_temp_workspace("turn-natural-linear-route");
  let (plugin_root, plugin) = create_linear_stdio_runner_plugin("turn-natural-linear-plugin");
  replace_plugin_catalog(&mut context, vec![plugin]);

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
        "title": "Natural Linear Thread"
      })),
    ),
  );

  let response = handle_request(
    &mut context,
    request(
      methods::TURN_START,
      Some(json!({
        "threadId": "thread-1",
        "message": "Prepare a Linear project update for this handoff."
      })),
    ),
  );

  remove_temp_workspace(&workspace);
  fs::remove_dir_all(&plugin_root).expect("cleanup linear plugin root");

  assert!(response.error.is_none());
  let result = response.result.expect("turn result");
  let items = result["items"].as_array().expect("items");
  assert_eq!(items[0]["kind"], "userMessage");
  assert_eq!(items[1]["kind"], "pluginCommand");
  assert_eq!(
    items[1]["attributes"]["commandId"],
    "linear-runner::linear.update"
  );
  assert_eq!(items[1]["attributes"]["agentToolKind"], "connector");
  assert_eq!(
    items[1]["attributes"]["toolPlanningMode"],
    "deterministicConnectorRanking"
  );
  assert_eq!(
    items[1]["attributes"]["toolPlanningSelectedCommandId"],
    "linear-runner::linear.update"
  );
  assert_eq!(items[1]["attributes"]["toolPlanningCandidateCount"], "1");
  assert_eq!(
    items[1]["attributes"]["toolPlanningSelectionState"],
    "deterministicSingle"
  );
  assert_eq!(items[2]["kind"], "pluginResult");
  assert!(items[2]["content"]
    .as_str()
    .expect("linear content")
    .contains("connector_id=true service=true input=true"));
  assert_eq!(
    items[2]["attributes"]["pluginRunnerConnectorId"],
    "linear-runner::linear"
  );
  assert_eq!(
    items[2]["attributes"]["pluginRunnerConnectorServices"],
    "linear"
  );
  assert_eq!(result["activeTurnId"], serde_json::Value::Null);
  assert_eq!(
    result["pendingApprovals"]
      .as_array()
      .expect("pending approvals")
      .len(),
    0
  );
  assert!(context
    .memory_state
    .recent_notes(16)
    .into_iter()
    .any(|note| note.title == "Linear Update Draft"));
}

#[cfg(unix)]
#[test]
fn turn_start_routes_plugin_shell_follow_up_to_approval() {
  let mut context = RuntimeContext::new_in_memory();
  let workspace = create_temp_workspace("turn-plugin-shell-follow-up");
  let output_json = r#"{
  "items": [
    {
      "kind": "pluginResult",
      "title": "Shell Follow-up",
      "content": "Request shell inspection through the agent loop.",
      "attributes": {
        "nextAction": "run_shell",
        "nextCommand": "git status --short"
      }
    }
  ]
}"#;
  let (plugin_root, plugin) = create_next_action_stdio_runner_plugin(
    "turn-plugin-shell-follow-up-plugin",
    "follow-shell",
    output_json,
    &["file.read", "shell.exec"],
  );
  replace_plugin_catalog(&mut context, vec![plugin]);

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
        "title": "Plugin Shell Follow-up Thread"
      })),
    ),
  );

  let response = handle_request(
    &mut context,
    request(
      methods::TURN_START,
      Some(json!({
        "threadId": "thread-1",
        "message": "/plugin follow-shell::follow-shell.run"
      })),
    ),
  );

  remove_temp_workspace(&workspace);
  fs::remove_dir_all(&plugin_root).expect("cleanup plugin root");

  assert!(response.error.is_none());
  let result = response.result.expect("turn result");
  let items = result["items"].as_array().expect("items");
  let plugin_observation = items
    .iter()
    .find(|item| item["title"] == "Shell Follow-up")
    .expect("plugin shell observation");
  assert_eq!(plugin_observation["attributes"]["nextAction"], "run_shell");
  let approval = items
    .iter()
    .find(|item| item["kind"] == "approvalRequested")
    .expect("shell approval");
  assert_eq!(approval["attributes"]["action"], "run_shell");
  assert_eq!(approval["attributes"]["command"], "git status --short");
  assert_eq!(
    approval["attributes"]["agentLoopStopReason"],
    "approvalPaused"
  );
  assert_eq!(result["activeTurnId"], serde_json::Value::Null);
  assert_eq!(
    result["pendingApprovals"]
      .as_array()
      .expect("approvals")
      .len(),
    1
  );
}

#[cfg(unix)]
#[test]
fn turn_start_routes_plugin_write_follow_up_to_diff_approval() {
  let mut context = RuntimeContext::new_in_memory();
  let workspace = create_temp_workspace("turn-plugin-write-follow-up");
  fs::write(workspace.join("README.md"), "Before\n").expect("write readme");
  let output_json = r#"{
  "items": [
    {
      "kind": "pluginResult",
      "title": "Write Follow-up",
      "content": "Request a file update through the agent loop.",
      "attributes": {
        "nextAction": "write_file",
        "nextRelativePath": "README.md",
        "nextContent": "After\n"
      }
    }
  ]
}"#;
  let (plugin_root, plugin) = create_next_action_stdio_runner_plugin(
    "turn-plugin-write-follow-up-plugin",
    "follow-write",
    output_json,
    &["file.read", "file.write"],
  );
  replace_plugin_catalog(&mut context, vec![plugin]);

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
        "title": "Plugin Write Follow-up Thread"
      })),
    ),
  );

  let response = handle_request(
    &mut context,
    request(
      methods::TURN_START,
      Some(json!({
        "threadId": "thread-1",
        "message": "/plugin follow-write::follow-write.run"
      })),
    ),
  );

  remove_temp_workspace(&workspace);
  fs::remove_dir_all(&plugin_root).expect("cleanup plugin root");

  assert!(response.error.is_none());
  let result = response.result.expect("turn result");
  let items = result["items"].as_array().expect("items");
  let plugin_observation = items
    .iter()
    .find(|item| item["title"] == "Write Follow-up")
    .expect("plugin write observation");
  assert_eq!(plugin_observation["attributes"]["nextAction"], "write_file");
  let diff = items
    .iter()
    .find(|item| item["kind"] == "diffArtifact")
    .expect("diff artifact");
  assert_eq!(diff["attributes"]["relativePath"], "README.md");
  let approval = items
    .iter()
    .find(|item| item["kind"] == "approvalRequested")
    .expect("write approval");
  assert_eq!(approval["attributes"]["action"], "write_file");
  assert_eq!(approval["attributes"]["relativePath"], "README.md");
  assert_eq!(
    approval["attributes"]["agentLoopStopReason"],
    "approvalPaused"
  );
  assert_eq!(result["activeTurnId"], serde_json::Value::Null);
  assert_eq!(
    result["pendingApprovals"]
      .as_array()
      .expect("approvals")
      .len(),
    1
  );
}

#[cfg(unix)]
#[test]
fn turn_start_routes_plugin_command_follow_up_through_loop() {
  let mut context = RuntimeContext::new_in_memory();
  let workspace = create_temp_workspace("turn-plugin-command-follow-up");
  let source_output_json = r#"{
  "items": [
    {
      "kind": "pluginResult",
      "title": "Plugin Command Follow-up",
      "content": "Request a second plugin command through the agent loop.",
      "attributes": {
        "nextAction": "plugin_command",
        "nextCommandId": "follow-target::follow-target.run",
        "nextCommandInput": "handoff payload"
      }
    }
  ],
  "memoryNotes": [
    {
      "title": "Source Handoff Memory",
      "body": "Source plugin memory should survive chained plugin execution.",
      "source": "plugin.follow-source",
      "tags": ["chain", "source"]
    }
  ]
}"#;
  let target_output_json = r#"{
  "content": "Target plugin received the handoff payload.",
  "memoryNotes": [
    {
      "title": "Target Handoff Memory",
      "body": "Target plugin memory should be captured after a chained command.",
      "source": "plugin.follow-target",
      "tags": ["chain", "target"]
    }
  ]
}"#;
  let (source_root, source_plugin) = create_next_action_stdio_runner_plugin(
    "turn-plugin-command-follow-up-source",
    "follow-source",
    source_output_json,
    &["file.read"],
  );
  let (target_root, target_plugin) = create_next_action_stdio_runner_plugin(
    "turn-plugin-command-follow-up-target",
    "follow-target",
    target_output_json,
    &["file.read"],
  );
  replace_plugin_catalog(&mut context, vec![source_plugin, target_plugin]);

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
        "title": "Plugin Command Follow-up Thread"
      })),
    ),
  );

  let response = handle_request(
    &mut context,
    request(
      methods::TURN_START,
      Some(json!({
        "threadId": "thread-1",
        "message": "/plugin follow-source::follow-source.run"
      })),
    ),
  );

  remove_temp_workspace(&workspace);
  fs::remove_dir_all(&source_root).expect("cleanup source plugin root");
  fs::remove_dir_all(&target_root).expect("cleanup target plugin root");

  assert!(response.error.is_none());
  let result = response.result.expect("turn result");
  let items = result["items"].as_array().expect("items");
  let source_observation = items
    .iter()
    .find(|item| item["title"] == "Plugin Command Follow-up")
    .expect("source plugin observation");
  assert_eq!(
    source_observation["attributes"]["nextAction"],
    "plugin_command"
  );
  assert_eq!(
    source_observation["attributes"]["nextCommandId"],
    "follow-target::follow-target.run"
  );
  let target_command = items
    .iter()
    .find(|item| {
      item["kind"] == "pluginCommand"
        && item["attributes"]["commandId"] == "follow-target::follow-target.run"
    })
    .expect("target plugin command");
  assert_eq!(target_command["attributes"]["agentStepIndex"], "2");
  assert_eq!(target_command["attributes"]["agentToolKind"], "plugin");
  let target_result = items
    .iter()
    .find(|item| {
      item["kind"] == "pluginResult"
        && item["attributes"]["commandId"] == "follow-target::follow-target.run"
    })
    .expect("target plugin result");
  assert!(target_result["content"]
    .as_str()
    .expect("target content")
    .contains("Target plugin received"));
  assert_eq!(
    target_result["attributes"]["agentLoopStopReason"],
    "completed"
  );
  assert_eq!(result["activeTurnId"], serde_json::Value::Null);
  assert_eq!(
    result["pendingApprovals"]
      .as_array()
      .expect("approvals")
      .len(),
    0
  );
  let memory_titles = context
    .memory_state
    .recent_notes(16)
    .into_iter()
    .map(|note| note.title)
    .collect::<Vec<_>>();
  assert!(memory_titles.contains(&"Source Handoff Memory".to_string()));
  assert!(memory_titles.contains(&"Target Handoff Memory".to_string()));
}

#[test]
fn bundled_builtin_plugin_commands_return_owned_results() {
  let mut context = RuntimeContext::new_in_memory();
  let workspace = create_temp_workspace("bundled-plugin-results");
  fs::write(workspace.join("README.md"), "# Bundled Connector Results\n").expect("write readme");
  replace_plugin_catalog(
    &mut context,
    vec![
      bundled_manifest_plugin_entry(
        "review-assistant",
        "Review Assistant",
        true,
        true,
        &["command:review.inspect-diff"],
        &["file.read", "model.invoke"],
      ),
      bundled_manifest_plugin_entry(
        "shell-recorder",
        "Shell Recorder",
        true,
        false,
        &["command:shell.summarize-session", "hook:shell.recorder"],
        &["shell.exec"],
      ),
    ],
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
        "title": "Bundled Plugin Thread"
      })),
    ),
  );

  let review_response = handle_request(
    &mut context,
    request(
      methods::PLUGIN_COMMAND_RUN,
      Some(json!({
        "threadId": "thread-1",
        "commandId": "review-assistant::review.inspect-diff"
      })),
    ),
  );
  let shell_response = handle_request(
    &mut context,
    request(
      methods::PLUGIN_COMMAND_RUN,
      Some(json!({
        "threadId": "thread-1",
        "commandId": "shell-recorder::shell.summarize-session"
      })),
    ),
  );

  remove_temp_workspace(&workspace);

  assert!(review_response.error.is_none());
  assert!(shell_response.error.is_none());
  let review_result = review_response.result.expect("review result");
  let shell_result = shell_response.result.expect("shell result");
  let review_items = review_result["items"]
    .as_array()
    .expect("review items")
    .clone();
  let shell_items = shell_result["items"]
    .as_array()
    .expect("shell items")
    .clone();
  assert_eq!(review_items[1]["kind"], "pluginResult");
  assert_eq!(
    review_items[1]["attributes"]["executionKind"],
    "builtin.reviewDiffSummary"
  );
  assert_eq!(shell_items[1]["kind"], "pluginResult");
  assert_eq!(
    shell_items[1]["attributes"]["executionKind"],
    "builtin.shellSessionSummary"
  );
}

#[test]
fn plugin_command_run_rejects_commands_without_execution_contract() {
  let mut context = RuntimeContext::new_in_memory();
  let source_root =
    create_temp_plugin_bundle("plugin-command-contract", "contractless", "Contractless");
  let workspace = create_temp_workspace("plugin-command-contract-workspace");
  let plugin_manifest = source_root.join("amentia-plugin.json");
  replace_plugin_catalog(
    &mut context,
    vec![temp_manifest_plugin_entry(
      "contractless",
      "Contractless",
      "Command plugin without an execution contract",
      &["command:contractless.run"],
      &["file.read"],
      &plugin_manifest,
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
        "title": "Plugin Contract Thread"
      })),
    ),
  );

  let response = handle_request(
    &mut context,
    request(
      methods::PLUGIN_COMMAND_RUN,
      Some(json!({
        "threadId": "thread-1",
        "commandId": "contractless::contractless.run"
      })),
    ),
  );

  remove_temp_workspace(&workspace);
  remove_temp_plugin_source(&source_root);

  let error = response.error.expect("command contract error");
  assert_eq!(error.code, -32053);
  assert!(error
    .message
    .contains("requires an explicit execution contract"));
  let data = error.data.expect("command readiness error data");
  assert_eq!(data["pluginId"], "contractless");
  assert_eq!(data["commandId"], "contractless::contractless.run");
  assert_eq!(data["runStatus"], "missingExecution");
  assert!(data["runRepairHint"]
    .as_str()
    .expect("repair hint")
    .contains("local runner"));
}

#[cfg(unix)]
#[test]
fn plugin_command_run_executes_bounded_stdio_runner() {
  let mut context = RuntimeContext::new_in_memory();
  let source_root = create_temp_plugin_bundle(
    "plugin-command-stdio-runner",
    "stdio-runner",
    "Stdio Runner",
  );
  let workspace = create_temp_workspace("plugin-command-stdio-workspace");
  let plugin_manifest = source_root.join("amentia-plugin.json");
  let runner_path = source_root.join("runner.sh");
  fs::write(
    source_root.join("commands").join("stdio-runner.run.json"),
    r#"{
  "title": "Run Stdio Plugin",
  "description": "Execute a local stdio runner.",
  "prompt": "Run the local plugin runner.",
  "execution": {
    "kind": "stdio.echo",
    "entrypoint": "runner.sh"
  }
}"#,
  )
  .expect("write command manifest");
  fs::write(
    &runner_path,
    r#"#!/bin/sh
cat >/dev/null
[ -n "$AMENTIA_PLUGIN_SANDBOX_TEMP" ] || exit 9
printf '{"content":"External runner completed."}\n'
"#,
  )
  .expect("write runner");
  make_test_file_executable(&runner_path, "stdio runner");
  replace_plugin_catalog(
    &mut context,
    vec![temp_manifest_plugin_entry(
      "stdio-runner",
      "Stdio Runner",
      "Stdio command plugin",
      &["command:stdio-runner.run"],
      &["file.read"],
      &plugin_manifest,
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
        "title": "Stdio Runner Thread"
      })),
    ),
  );

  let response = handle_request(
    &mut context,
    request(
      methods::PLUGIN_COMMAND_RUN,
      Some(json!({
        "threadId": "thread-1",
        "commandId": "stdio-runner::stdio-runner.run",
        "input": "Run now"
      })),
    ),
  );

  remove_temp_workspace(&workspace);
  remove_temp_plugin_source(&source_root);

  assert!(response.error.is_none());
  let result = response.result.expect("command run result");
  let items = result["items"].as_array().expect("items");
  assert_eq!(items[1]["kind"], "pluginResult");
  assert_eq!(items[1]["attributes"]["executionKind"], "stdio.echo");
  assert_eq!(items[1]["attributes"]["sandboxMode"], "workspaceReadWrite");
  assert!(items[1]["attributes"]["sandboxBackend"].is_string());
  assert!(items[1]["attributes"]["sandboxAvailable"].is_string());
  assert!(items[1]["attributes"]["sandboxTempRoot"].is_string());
  assert_eq!(items[1]["attributes"]["sandboxNetworkAllowed"], "false");
  assert_eq!(
    items[1]["attributes"]["pluginRunnerExitReason"],
    "completed"
  );
  assert_eq!(items[1]["content"], "External runner completed.");
  assert_eq!(
    result["pendingApprovals"]
      .as_array()
      .expect("pending approvals")
      .len(),
    0
  );
  assert_eq!(
    context
      .execution_state
      .counts()
      .running_plugin_command_count(),
    0
  );
}

#[cfg(unix)]
#[test]
fn plugin_command_run_rejects_missing_required_user_input_before_runner_start() {
  let mut context = RuntimeContext::new_in_memory();
  let source_root = create_temp_plugin_bundle(
    "plugin-command-required-input",
    "required-input",
    "Required Input",
  );
  let workspace = create_temp_workspace("plugin-command-required-input-workspace");
  let plugin_manifest = source_root.join("amentia-plugin.json");
  let runner_path = source_root.join("runner.sh");
  fs::write(
    source_root.join("commands").join("required-input.run.json"),
    r#"{
  "title": "Run Required Input Plugin",
  "description": "Execute a local stdio runner that requires user input.",
  "prompt": "Run the local plugin runner.",
  "execution": {
    "kind": "stdio.requiredInput",
    "entrypoint": "runner.sh",
    "input": {
      "envelope": "amentia.plugin.command.input",
      "fields": [
        {
          "name": "threadId",
          "kind": "string",
          "required": true
        },
        {
          "name": "input",
          "kind": "text",
          "required": true
        }
      ]
    }
  }
}"#,
  )
  .expect("write command manifest");
  fs::write(
    &runner_path,
    r#"#!/bin/sh
printf '{"content":"runner should not start"}\n'
"#,
  )
  .expect("write runner");
  make_test_file_executable(&runner_path, "required input runner");
  replace_plugin_catalog(
    &mut context,
    vec![temp_manifest_plugin_entry(
      "required-input",
      "Required Input",
      "Required input command plugin",
      &["command:required-input.run"],
      &["file.read"],
      &plugin_manifest,
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
        "title": "Required Input Thread"
      })),
    ),
  );

  let response = handle_request(
    &mut context,
    request(
      methods::PLUGIN_COMMAND_RUN,
      Some(json!({
        "threadId": "thread-1",
        "commandId": "required-input::required-input.run"
      })),
    ),
  );

  remove_temp_workspace(&workspace);
  remove_temp_plugin_source(&source_root);

  let error = response.error.expect("required input error");
  assert_eq!(error.code, -32053);
  assert!(error
    .message
    .contains("requires command input field `input`"));
  let data = error.data.expect("input contract error data");
  assert_eq!(data["pluginId"], "required-input");
  assert_eq!(data["commandId"], "required-input::required-input.run");
  assert_eq!(
    data["sourcePath"],
    source_root
      .join("commands")
      .join("required-input.run.json")
      .display()
      .to_string()
  );
  assert_eq!(data["runStatus"], "missingInput");
  assert!(data["runBlocker"]
    .as_str()
    .expect("run blocker")
    .contains("requires command input field `input`"));
  assert!(data["runRepairHint"]
    .as_str()
    .expect("repair hint")
    .contains("Run the command with input"));
  assert_eq!(
    context
      .execution_state
      .counts()
      .running_plugin_command_count(),
    0
  );
}

#[cfg(unix)]
#[test]
fn plugin_command_run_disables_network_for_empty_connector_scope() {
  let mut context = RuntimeContext::new_in_memory();
  let source_root = create_temp_plugin_bundle(
    "plugin-command-empty-connector-scope",
    "notion-runner",
    "Notion Runner",
  );
  let workspace = create_temp_workspace("plugin-command-empty-connector-scope-workspace");
  let plugin_manifest = source_root.join("amentia-plugin.json");
  let runner_path = source_root.join("runner.sh");
  fs::write(
    &plugin_manifest,
    r#"{
  "name": "notion-runner",
  "version": "0.1.0",
  "displayName": "Notion Runner",
  "description": "Connector plugin with a local status command",
  "author": { "name": "Amentia" },
  "capabilities": ["command:notion-runner.status", "connector:notion"],
  "permissions": ["network.outbound"],
  "appConnectors": [
    {
      "id": "notion",
      "displayName": "Notion",
      "service": "notion"
    }
  ],
  "authPolicy": {
    "type": "oauth2",
    "required": true,
    "scopes": ["read_content"],
    "credentialStore": "local"
  },
  "defaultEnabled": true
}"#,
  )
  .expect("write connector plugin manifest");
  fs::write(
    source_root
      .join("commands")
      .join("notion-runner.status.json"),
    r#"{
  "title": "Show Notion Status",
  "description": "Run local setup checks without contacting Notion.",
  "prompt": "Show local status.",
  "execution": {
    "kind": "stdio.status",
    "entrypoint": "runner.sh",
    "connectors": []
  }
}"#,
  )
  .expect("write status command manifest");
  fs::write(
    &runner_path,
    r#"#!/bin/sh
cat >/dev/null
printf '{"content":"status ok"}\n'
"#,
  )
  .expect("write runner");
  make_test_file_executable(&runner_path, "connector status runner");
  replace_plugin_catalog(
    &mut context,
    vec![temp_manifest_plugin_entry(
      "notion-runner",
      "Notion Runner",
      "Connector plugin with a local status command",
      &["command:notion-runner.status", "connector:notion"],
      &["network.outbound"],
      &plugin_manifest,
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
        "title": "Connector Status Thread"
      })),
    ),
  );

  let response = handle_request(
    &mut context,
    request(
      methods::PLUGIN_COMMAND_RUN,
      Some(json!({
        "threadId": "thread-1",
        "commandId": "notion-runner::notion-runner.status"
      })),
    ),
  );

  remove_temp_workspace(&workspace);
  remove_temp_plugin_source(&source_root);

  assert!(response.error.is_none());
  let result = response.result.expect("command run result");
  let items = result["items"].as_array().expect("items");
  assert_eq!(items[1]["kind"], "pluginResult");
  assert_eq!(items[1]["attributes"]["sandboxNetworkAllowed"], "false");
  assert!(items[1]["attributes"]["sandboxNetworkPolicy"]
    .as_str()
    .expect("sandbox network policy")
    .contains("network denied"));
  assert_eq!(items[1]["content"], "status ok");
  assert_eq!(
    result["pendingApprovals"]
      .as_array()
      .expect("pending approvals")
      .len(),
    0
  );
}

#[cfg(unix)]
#[test]
fn plugin_command_run_passes_auth_free_connector_context_to_runner() {
  let mut context = RuntimeContext::new_in_memory();
  let source_root = create_temp_plugin_bundle(
    "plugin-command-auth-free-connector",
    "browser-runner",
    "Browser Runner",
  );
  let workspace = create_temp_workspace("plugin-command-auth-free-connector-workspace");
  let plugin_manifest = source_root.join("amentia-plugin.json");
  let runner_path = source_root.join("runner.sh");
  fs::write(
    &plugin_manifest,
    r#"{
  "name": "browser-runner",
  "version": "0.1.0",
  "displayName": "Browser Runner",
  "description": "Auth-free connector command plugin",
  "author": { "name": "Amentia" },
  "capabilities": ["command:browser-runner.search", "connector:web"],
  "permissions": ["network.outbound"],
  "appConnectors": [
    {
      "id": "web",
      "displayName": "Web",
      "service": "web"
    }
  ],
  "authPolicy": {
    "type": "none",
    "required": false,
    "credentialStore": "none"
  },
  "defaultEnabled": true
}"#,
  )
  .expect("write auth-free connector plugin manifest");
  fs::write(
    source_root
      .join("commands")
      .join("browser-runner.search.json"),
    r#"{
  "title": "Search Web",
  "description": "Run an auth-free connector-backed stdio command.",
  "prompt": "Search the web.",
  "execution": {
    "kind": "stdio.webSearch",
    "entrypoint": "runner.sh",
    "connectors": ["web"]
  }
}"#,
  )
  .expect("write auth-free connector command manifest");
  fs::write(
    &runner_path,
    r#"#!/bin/sh
payload=$(cat)
case "$payload" in *'"connectorId":"browser-runner::web"'*) connector_id=true;; *) connector_id=false;; esac
case "$payload" in *'"provider":"amentia.noCredentialRequired"'*) provider=true;; *) provider=false;; esac
case "$payload" in *'"handle":"browser-runner::web"'*) handle=true;; *) handle=false;; esac
case "$payload" in *'"store":"none"'*) store=true;; *) store=false;; esac
case "$payload" in *'"envKey"'*) env_key=true;; *) env_key=false;; esac
case "$payload" in *"access_token"*|*"refresh_token"*|*"secret"*) secret_leak=true;; *) secret_leak=false;; esac
printf '{"content":"connectorId=%s provider=%s handle=%s store=%s envKey=%s secretLeak=%s"}\n' "$connector_id" "$provider" "$handle" "$store" "$env_key" "$secret_leak"
"#,
  )
  .expect("write auth-free connector runner");
  make_test_file_executable(&runner_path, "auth-free connector runner");
  replace_plugin_catalog(
    &mut context,
    vec![temp_manifest_plugin_entry(
      "browser-runner",
      "Browser Runner",
      "Auth-free connector command plugin",
      &["command:browser-runner.search", "connector:web"],
      &["network.outbound"],
      &plugin_manifest,
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
        "title": "Auth-Free Connector Thread"
      })),
    ),
  );

  let response = handle_request(
    &mut context,
    request(
      methods::PLUGIN_COMMAND_RUN,
      Some(json!({
        "threadId": "thread-1",
        "commandId": "browser-runner::browser-runner.search"
      })),
    ),
  );

  remove_temp_workspace(&workspace);
  remove_temp_plugin_source(&source_root);

  assert!(response.error.is_none());
  let result = response.result.expect("command run result");
  let items = result["items"].as_array().expect("items");
  assert_eq!(items[0]["kind"], "pluginCommand");
  assert_eq!(
    items[0]["attributes"]["connectorIds"],
    "browser-runner::web"
  );
  assert_eq!(items[0]["attributes"]["connectorServices"], "web");
  assert_eq!(
    items[0]["attributes"]["connectorCredentialProviders"],
    "amentia.noCredentialRequired"
  );
  assert_eq!(items[0]["attributes"]["connectorSecretBindings"], "none");
  assert_eq!(items[1]["kind"], "pluginResult");
  assert_eq!(
    items[1]["content"],
    "connectorId=true provider=true handle=true store=true envKey=false secretLeak=false"
  );
  assert_eq!(
    result["pendingApprovals"]
      .as_array()
      .expect("pending approvals")
      .len(),
    0
  );
}

#[cfg(unix)]
#[test]
fn plugin_command_run_approves_connector_stdio_runner_without_secrets() {
  let mut context = RuntimeContext::new_in_memory();
  let (source_root, catalog_entry) =
    create_notion_stdio_runner_plugin("plugin-command-connector-runner");
  let workspace = create_temp_workspace("plugin-command-connector-runner-workspace");
  replace_plugin_catalog(&mut context, vec![catalog_entry]);

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
        "title": "Connector Runner Thread"
      })),
    ),
  );
  let authorize_response = handle_request(
    &mut context,
    request(
      methods::PLUGIN_CONNECTOR_AUTHORIZE,
      Some(json!({
        "connectorId": "notion-runner::notion"
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
        "commandId": "notion-runner::notion-runner.sync"
      })),
    ),
  );

  assert!(response.error.is_none());
  let result = response.result.expect("command run result");
  let items = result["items"].as_array().expect("items");
  assert_eq!(items[0]["kind"], "pluginCommand");
  assert_eq!(
    items[0]["attributes"]["connectorIds"],
    "notion-runner::notion"
  );
  assert!(items[0]["attributes"]
    .get("connectorCredentialHandles")
    .is_none());
  assert_eq!(
    items[0]["attributes"]["connectorSecretBindings"],
    "marker-only"
  );
  assert_eq!(items[1]["kind"], "approvalRequested");
  assert_eq!(
    items[1]["attributes"]["connectorId"],
    "notion-runner::notion"
  );
  assert_eq!(items[1]["attributes"]["executionKind"], "stdio.notionSync");
  assert_eq!(
    items[1]["attributes"]["connectorSecretBindings"],
    "marker-only"
  );
  assert_eq!(
    result["pendingApprovals"][0]["action"],
    "run_plugin_command"
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

  remove_temp_workspace(&workspace);
  remove_temp_plugin_source(&source_root);

  assert!(approval_response.error.is_none());
  let approval_result = approval_response.result.expect("approval result");
  let items = approval_result["items"].as_array().expect("approval items");
  assert_eq!(items[0]["kind"], "approvalResolved");
  assert_eq!(items[0]["attributes"]["decision"], "approved");
  assert_eq!(
    items[0]["attributes"]["commandId"],
    "notion-runner::notion-runner.sync"
  );
  assert_eq!(items[1]["kind"], "pluginCommand");
  assert_eq!(
    items[1]["attributes"]["connectorIds"],
    "notion-runner::notion"
  );
  assert_eq!(
    items[1]["attributes"]["connectorSecretBindings"],
    "marker-only"
  );
  assert_eq!(items[2]["kind"], "pluginResult");
  assert!(items[2]["attributes"]
    .get("pluginRunnerCredentialHandles")
    .is_none());
  assert_eq!(
    items[2]["attributes"]["pluginRunnerExecutionKind"],
    "stdio.notionSync"
  );
  assert_eq!(
    items[2]["attributes"]["pluginRunnerSecretBindings"],
    "marker-only"
  );
  assert_eq!(
    items[2]["content"],
    "connectorId=true provider=true handle=true store=true label=true secretLeak=false"
  );
  let memory_item = items
    .iter()
    .find(|item| item["title"] == "Plugin Memory Note Saved")
    .expect("approved runner memory item");
  assert_eq!(
    memory_item["attributes"]["memoryNoteTitle"],
    "Approved Connector Memory"
  );
  let saved_note = context
    .memory_state
    .recent_notes(16)
    .into_iter()
    .find(|note| note.title == "Approved Connector Memory")
    .expect("saved approved runner memory note");
  assert_eq!(saved_note.source, "plugin.notion-runner.approved");
  assert!(saved_note
    .body
    .contains("Connector runner memory survives approval execution."));
}

#[cfg(unix)]
#[test]
fn approval_respond_returns_structured_plugin_command_readiness_error() {
  let mut context = RuntimeContext::new_in_memory();
  let (source_root, catalog_entry) =
    create_notion_stdio_runner_plugin("plugin-command-approval-readiness-error");
  let workspace = create_temp_workspace("plugin-command-approval-readiness-error-workspace");
  replace_plugin_catalog(&mut context, vec![catalog_entry.clone()]);

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
        "title": "Connector Runner Thread"
      })),
    ),
  );
  let authorize_response = handle_request(
    &mut context,
    request(
      methods::PLUGIN_CONNECTOR_AUTHORIZE,
      Some(json!({
        "connectorId": "notion-runner::notion"
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
        "commandId": "notion-runner::notion-runner.sync"
      })),
    ),
  );
  assert!(response.error.is_none());
  let result = response.result.expect("command run result");
  let approval_id = result["pendingApprovals"][0]["id"]
    .as_str()
    .expect("approval id")
    .to_string();

  let mut missing_permission_entry = catalog_entry;
  missing_permission_entry.permissions = vec!["mcp.connect".to_string()];
  replace_plugin_catalog(&mut context, vec![missing_permission_entry]);

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

  remove_temp_workspace(&workspace);
  remove_temp_plugin_source(&source_root);

  let error = approval_response
    .error
    .expect("approval-time plugin readiness error");
  assert_eq!(error.code, -32053);
  let data = error.data.expect("structured plugin command error data");
  assert_eq!(data["pluginId"], "notion-runner");
  assert_eq!(data["commandId"], "notion-runner::notion-runner.sync");
  assert_eq!(data["runStatus"], "missingPermission");
  assert!(data["runBlocker"]
    .as_str()
    .expect("run blocker")
    .contains("network.outbound"));
  assert!(data["runBlocker"]
    .as_str()
    .expect("run blocker")
    .contains("plugin action"));
  assert!(data["runRepairHint"]
    .as_str()
    .expect("repair hint")
    .contains("required permission"));
}

#[cfg(unix)]
#[test]
fn plugin_command_run_executes_mcp_stdio_connector_action() {
  let mut context = RuntimeContext::new_in_memory();
  let source_root =
    create_temp_plugin_bundle("plugin-command-mcp-runner", "notion-mcp", "Notion MCP");
  let workspace = create_temp_workspace("plugin-command-mcp-workspace");
  let plugin_manifest = source_root.join("amentia-plugin.json");
  let server_path = source_root.join("mcp-server.sh");
  fs::write(
    &plugin_manifest,
    r#"{
  "name": "notion-mcp",
  "version": "0.1.0",
  "displayName": "Notion MCP",
  "description": "Connector-backed MCP command plugin",
  "author": { "name": "Amentia" },
  "capabilities": ["command:notion-mcp.create-task", "mcp_server:notion", "connector:notion"],
  "permissions": ["network.outbound", "mcp.connect"],
  "mcpServers": [
    {
      "id": "notion",
      "command": "mcp-server.sh",
      "transport": "stdio"
    }
  ],
  "appConnectors": [
    {
      "id": "notion",
      "displayName": "Notion",
      "service": "notion",
      "homepage": "https://www.notion.so"
    }
  ],
  "authPolicy": {
    "type": "oauth2",
    "required": true,
    "scopes": ["insert_content"],
    "credentialStore": "local"
  },
  "defaultEnabled": true
}"#,
  )
  .expect("write mcp plugin manifest");
  fs::write(
    source_root
      .join("commands")
      .join("notion-mcp.create-task.json"),
    r#"{
  "title": "Create Notion Task",
  "description": "Create a Notion task through an MCP server.",
  "prompt": "Create a task in Notion from the current thread.",
  "execution": {
    "kind": "mcp.notionCreateTask",
    "driver": "mcp",
    "entrypoint": "notion.createTask"
  }
}"#,
  )
  .expect("write mcp command manifest");
  fs::write(
    &server_path,
    r#"#!/bin/sh
payload=$(cat)
case "$payload" in *'"method":"tools/call"'*) method=true;; *) method=false;; esac
case "$payload" in *'"name":"createTask"'*) tool=true;; *) tool=false;; esac
case "$payload" in *'"provider":"amentia.localCredentialProvider"'*) provider=true;; *) provider=false;; esac
case "$payload" in *'"handle":"notion-mcp::notion"'*) handle=true;; *) handle=false;; esac
case "$payload" in *"access_token"*|*"refresh_token"*|*"secret"*) secret_leak=true;; *) secret_leak=false;; esac
if [ "$AMENTIA_PLUGIN_CREDENTIAL_1_NOTION_MCP__NOTION" = "notion-local-token" ]; then credential_env=true; else credential_env=false; fi
case "$payload" in *"notion-local-token"*) token_leak=true;; *) token_leak=false;; esac
printf '{"jsonrpc":"2.0","id":1,"result":{}}\n'
printf '{"jsonrpc":"2.0","id":2,"result":{"content":[{"type":"text","text":"method=%s tool=%s provider=%s handle=%s secretLeak=%s credentialEnv=%s tokenLeak=%s"}]}}\n' "$method" "$tool" "$provider" "$handle" "$secret_leak" "$credential_env" "$token_leak"
"#,
  )
  .expect("write mcp server");
  make_test_file_executable(&server_path, "mcp server");
  replace_plugin_catalog(
    &mut context,
    vec![temp_manifest_plugin_entry(
      "notion-mcp",
      "Notion MCP",
      "Connector-backed MCP command plugin",
      &[
        "command:notion-mcp.create-task",
        "mcp_server:notion",
        "connector:notion",
      ],
      &["network.outbound", "mcp.connect"],
      &plugin_manifest,
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
        "title": "MCP Connector Thread"
      })),
    ),
  );
  let authorize_response = handle_request(
    &mut context,
    request(
      methods::PLUGIN_CONNECTOR_AUTHORIZE,
      Some(json!({
        "connectorId": "notion-mcp::notion",
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
        "commandId": "notion-mcp::notion-mcp.create-task",
        "input": "Create a follow-up task"
      })),
    ),
  );

  assert!(response.error.is_none());
  let result = response.result.expect("command run result");
  let items = result["items"].as_array().expect("items");
  assert_eq!(items[0]["kind"], "pluginCommand");
  assert_eq!(
    items[0]["attributes"]["pluginCommandRunId"],
    "thread-1::notion-mcp::notion-mcp.create-task"
  );
  assert_eq!(items[1]["kind"], "approvalRequested");
  assert_eq!(items[1]["title"], "Plugin Action Approval Requested");
  assert_eq!(
    items[1]["attributes"]["pluginCommandRunId"],
    items[0]["attributes"]["pluginCommandRunId"]
  );
  assert_eq!(items[1]["attributes"]["connectorId"], "notion-mcp::notion");
  assert_eq!(items[1]["attributes"]["connectorServices"], "notion");
  assert_eq!(
    items[1]["attributes"]["executionKind"],
    "mcp.notionCreateTask"
  );
  assert_eq!(
    items[1]["attributes"]["commandInput"],
    "Create a follow-up task"
  );
  assert_eq!(
    items[1]["attributes"]["connectorCredentialProviders"],
    "amentia.localCredentialProvider"
  );
  assert!(items[1]["attributes"]
    .get("connectorCredentialHandles")
    .is_none());
  assert_eq!(
    items[1]["attributes"]["connectorSecretBindings"],
    "env-bound"
  );
  assert!(items[1]["content"]
    .as_str()
    .expect("approval content")
    .contains("bindings env-bound"));
  assert_eq!(
    result["pendingApprovals"][0]["action"],
    "run_plugin_command"
  );
  assert_eq!(
    result["pendingApprovals"][0]["title"],
    "Run Create Notion Task"
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

  remove_temp_workspace(&workspace);
  remove_temp_plugin_source(&source_root);

  assert!(approval_response.error.is_none());
  let approval_result = approval_response.result.expect("approval result");
  let items = approval_result["items"].as_array().expect("approval items");
  assert_eq!(items[0]["kind"], "approvalResolved");
  assert_eq!(items[0]["attributes"]["decision"], "approved");
  assert_eq!(
    items[0]["attributes"]["commandId"],
    "notion-mcp::notion-mcp.create-task"
  );
  assert_eq!(items[1]["kind"], "pluginCommand");
  assert_eq!(items[2]["kind"], "pluginResult");
  assert_eq!(
    items[2]["attributes"]["executionKind"],
    "mcp.notionCreateTask"
  );
  assert_eq!(items[2]["attributes"]["mcpToolName"], "createTask");
  assert_eq!(
    items[2]["attributes"]["pluginRunnerSecretBindings"],
    "env-bound"
  );
  assert_eq!(
    items[2]["content"],
    "method=true tool=true provider=true handle=true secretLeak=false credentialEnv=true tokenLeak=false"
  );
}

#[cfg(unix)]
#[test]
fn plugin_command_run_accepts_mcp_structured_amentia_output() {
  let mut context = RuntimeContext::new_in_memory();
  let source_root = create_temp_plugin_bundle(
    "plugin-command-mcp-structured-output",
    "mcp-structured",
    "MCP Structured",
  );
  let workspace = create_temp_workspace("plugin-command-mcp-structured-workspace");
  let plugin_manifest = source_root.join("amentia-plugin.json");
  let server_path = source_root.join("mcp-server.sh");
  fs::write(
    &plugin_manifest,
    r#"{
  "name": "mcp-structured",
  "version": "0.1.0",
  "displayName": "MCP Structured",
  "description": "MCP command plugin with Amentia structured output",
  "author": { "name": "Amentia" },
  "capabilities": ["command:mcp-structured.capture", "mcp_server:local"],
  "permissions": ["mcp.connect"],
  "mcpServers": [
    {
      "id": "local",
      "command": "mcp-server.sh",
      "transport": "stdio"
    }
  ],
  "defaultEnabled": true
}"#,
  )
  .expect("write mcp structured plugin manifest");
  fs::write(
    source_root
      .join("commands")
      .join("mcp-structured.capture.json"),
    r#"{
  "title": "Capture MCP Context",
  "description": "Return Amentia timeline and memory output through MCP structured content.",
  "prompt": "Capture MCP context.",
  "execution": {
    "kind": "mcp.localCapture",
    "driver": "mcp",
    "entrypoint": "local.capture"
  }
}"#,
  )
  .expect("write mcp structured command manifest");
  fs::write(
    &server_path,
    r#"#!/bin/sh
cat >/dev/null
printf '{"jsonrpc":"2.0","id":1,"result":{}}\n'
cat <<'JSON'
{"jsonrpc":"2.0","id":2,"result":{"structuredContent":{"items":[{"kind":"pluginResult","title":"MCP Structured Item","content":"Owned MCP timeline item.","attributes":{"runner":"mcp"}}],"memoryNotes":[{"title":"MCP Structured Memory","body":"MCP structured content can store Amentia memory.","source":"plugin.mcp-structured","tags":["mcp","structured"]}]}}}
JSON
"#,
  )
  .expect("write mcp structured server");
  make_test_file_executable(&server_path, "mcp structured server");
  replace_plugin_catalog(
    &mut context,
    vec![temp_manifest_plugin_entry(
      "mcp-structured",
      "MCP Structured",
      "MCP command plugin with Amentia structured output",
      &["command:mcp-structured.capture", "mcp_server:local"],
      &["mcp.connect"],
      &plugin_manifest,
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
        "title": "MCP Structured Thread"
      })),
    ),
  );

  let response = handle_request(
    &mut context,
    request(
      methods::PLUGIN_COMMAND_RUN,
      Some(json!({
        "threadId": "thread-1",
        "commandId": "mcp-structured::mcp-structured.capture"
      })),
    ),
  );

  remove_temp_workspace(&workspace);
  remove_temp_plugin_source(&source_root);

  assert!(response.error.is_none());
  let result = response.result.expect("command run result");
  let items = result["items"].as_array().expect("items");
  assert_eq!(items[0]["kind"], "pluginCommand");
  assert_eq!(items[1]["title"], "MCP Structured Item");
  assert_eq!(items[1]["attributes"]["runner"], "mcp");
  assert_eq!(items[1]["attributes"]["mcpProtocolStatus"], "completed");
  assert_eq!(
    items[1]["attributes"]["mcpStructuredContentStatus"],
    "amentiaOutputEnvelope"
  );
  assert_eq!(
    items[1]["attributes"]["pluginRunnerOutputStatus"],
    "envelope"
  );
  assert_eq!(
    items[1]["attributes"]["pluginRunnerOutputValidTimelineItemCount"],
    "1"
  );
  assert_eq!(
    items[1]["attributes"]["pluginRunnerOutputMemoryNoteCount"],
    "1"
  );
  let memory_item = items
    .iter()
    .find(|item| item["title"] == "Plugin Memory Note Saved")
    .expect("mcp structured memory item");
  assert_eq!(
    memory_item["attributes"]["memoryNoteTitle"],
    "MCP Structured Memory"
  );
  let saved_note = context
    .memory_state
    .recent_notes(16)
    .into_iter()
    .find(|note| note.title == "MCP Structured Memory")
    .expect("saved mcp structured memory note");
  assert_eq!(saved_note.source, "plugin.mcp-structured");
  assert!(saved_note
    .body
    .contains("MCP structured content can store Amentia memory."));
}

#[cfg(unix)]
#[test]
fn plugin_command_run_accepts_mcp_text_amentia_output() {
  let mut context = RuntimeContext::new_in_memory();
  let source_root = create_temp_plugin_bundle(
    "plugin-command-mcp-text-amentia-output",
    "mcp-text",
    "MCP Text",
  );
  let workspace = create_temp_workspace("plugin-command-mcp-text-workspace");
  let plugin_manifest = source_root.join("amentia-plugin.json");
  let server_path = source_root.join("mcp-server.sh");
  fs::write(
    &plugin_manifest,
    r#"{
  "name": "mcp-text",
  "version": "0.1.0",
  "displayName": "MCP Text",
  "description": "MCP command plugin with text output",
  "author": { "name": "Amentia" },
  "capabilities": ["command:mcp-text.capture", "mcp_server:local"],
  "permissions": ["mcp.connect"],
  "mcpServers": [
    {
      "id": "local",
      "command": "mcp-server.sh",
      "transport": "stdio"
    }
  ],
  "defaultEnabled": true
}"#,
  )
  .expect("write mcp text plugin manifest");
  fs::write(
    source_root.join("commands").join("mcp-text.capture.json"),
    r#"{
  "title": "Capture MCP Text",
  "description": "Return a Amentia envelope from MCP text content.",
  "prompt": "Capture MCP text content.",
  "execution": {
    "kind": "mcp.localCapture",
    "driver": "mcp",
    "entrypoint": "local.capture"
  }
}"#,
  )
  .expect("write mcp text command manifest");
  fs::write(
    &server_path,
    r#"#!/bin/sh
cat >/dev/null
printf '{"jsonrpc":"2.0","id":1,"result":{}}\n'
cat <<'JSON'
{"jsonrpc":"2.0","id":2,"result":{"content":[{"type":"text","text":"{\"content\":\"MCP text envelope captured.\"}"}]}}
JSON
"#,
  )
  .expect("write mcp text server");
  make_test_file_executable(&server_path, "mcp text server");
  replace_plugin_catalog(
    &mut context,
    vec![temp_manifest_plugin_entry(
      "mcp-text",
      "MCP Text",
      "MCP command plugin with text output",
      &["command:mcp-text.capture", "mcp_server:local"],
      &["mcp.connect"],
      &plugin_manifest,
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
        "title": "MCP Text Thread"
      })),
    ),
  );

  let response = handle_request(
    &mut context,
    request(
      methods::PLUGIN_COMMAND_RUN,
      Some(json!({
        "threadId": "thread-1",
        "commandId": "mcp-text::mcp-text.capture"
      })),
    ),
  );

  remove_temp_workspace(&workspace);
  remove_temp_plugin_source(&source_root);

  assert!(response.error.is_none());
  let result = response.result.expect("command run result");
  let items = result["items"].as_array().expect("items");
  assert_eq!(items[0]["kind"], "pluginCommand");
  assert_eq!(items[1]["kind"], "pluginResult");
  assert_eq!(items[1]["content"], "MCP text envelope captured.");
  assert_eq!(items[1]["attributes"]["mcpProtocolStatus"], "completed");
  assert_eq!(
    items[1]["attributes"]["mcpContentStatus"],
    "amentiaOutputEnvelope"
  );
  assert_eq!(
    items[1]["attributes"]["pluginRunnerOutputStatus"],
    "envelope"
  );
  assert_eq!(items[1]["attributes"]["pluginRunnerOutputParsed"], "true");
}

#[cfg(unix)]
#[test]
fn plugin_command_run_preserves_generic_mcp_structured_content() {
  let mut context = RuntimeContext::new_in_memory();
  let source_root = create_temp_plugin_bundle(
    "plugin-command-mcp-generic-structured-output",
    "mcp-generic",
    "MCP Generic",
  );
  let workspace = create_temp_workspace("plugin-command-mcp-generic-workspace");
  let plugin_manifest = source_root.join("amentia-plugin.json");
  let server_path = source_root.join("mcp-server.sh");
  fs::write(
    &plugin_manifest,
    r#"{
  "name": "mcp-generic",
  "version": "0.1.0",
  "displayName": "MCP Generic",
  "description": "MCP command plugin with generic structured output",
  "author": { "name": "Amentia" },
  "capabilities": ["command:mcp-generic.inspect", "mcp_server:local"],
  "permissions": ["mcp.connect"],
  "mcpServers": [
    {
      "id": "local",
      "command": "mcp-server.sh",
      "transport": "stdio"
    }
  ],
  "defaultEnabled": true
}"#,
  )
  .expect("write mcp generic plugin manifest");
  fs::write(
    source_root
      .join("commands")
      .join("mcp-generic.inspect.json"),
    r#"{
  "title": "Inspect MCP Data",
  "description": "Return generic MCP structured content.",
  "prompt": "Inspect MCP data.",
  "execution": {
    "kind": "mcp.localInspect",
    "driver": "mcp",
    "entrypoint": "local.inspect"
  }
}"#,
  )
  .expect("write mcp generic command manifest");
  fs::write(
    &server_path,
    r#"#!/bin/sh
cat >/dev/null
printf '{"jsonrpc":"2.0","id":1,"result":{}}\n'
cat <<'JSON'
{"jsonrpc":"2.0","id":2,"result":{"structuredContent":{"content":{"pageId":"abc123"},"status":"ok"}}}
JSON
"#,
  )
  .expect("write mcp generic server");
  make_test_file_executable(&server_path, "mcp generic server");
  replace_plugin_catalog(
    &mut context,
    vec![temp_manifest_plugin_entry(
      "mcp-generic",
      "MCP Generic",
      "MCP command plugin with generic structured output",
      &["command:mcp-generic.inspect", "mcp_server:local"],
      &["mcp.connect"],
      &plugin_manifest,
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
        "title": "MCP Generic Thread"
      })),
    ),
  );

  let response = handle_request(
    &mut context,
    request(
      methods::PLUGIN_COMMAND_RUN,
      Some(json!({
        "threadId": "thread-1",
        "commandId": "mcp-generic::mcp-generic.inspect"
      })),
    ),
  );

  remove_temp_workspace(&workspace);
  remove_temp_plugin_source(&source_root);

  assert!(response.error.is_none());
  let result = response.result.expect("command run result");
  let items = result["items"].as_array().expect("items");
  assert_eq!(items[1]["kind"], "pluginResult");
  assert_eq!(items[1]["attributes"]["mcpProtocolStatus"], "completed");
  assert_eq!(
    items[1]["attributes"]["mcpStructuredContentStatus"],
    "generic"
  );
  assert!(items[1]["attributes"]["pluginRunnerOutputStatus"].is_null());
  assert!(items[1]["content"]
    .as_str()
    .expect("generic structured content")
    .contains("\"pageId\": \"abc123\""));
}

#[cfg(unix)]
#[test]
fn plugin_command_run_classifies_malformed_mcp_amentia_output() {
  let mut context = RuntimeContext::new_in_memory();
  let source_root = create_temp_plugin_bundle(
    "plugin-command-mcp-malformed-amentia-output",
    "mcp-malformed",
    "MCP Malformed",
  );
  let workspace = create_temp_workspace("plugin-command-mcp-malformed-workspace");
  let plugin_manifest = source_root.join("amentia-plugin.json");
  let server_path = source_root.join("mcp-server.sh");
  fs::write(
    &plugin_manifest,
    r#"{
  "name": "mcp-malformed",
  "version": "0.1.0",
  "displayName": "MCP Malformed",
  "description": "MCP command plugin with malformed Amentia structured output",
  "author": { "name": "Amentia" },
  "capabilities": ["command:mcp-malformed.inspect", "mcp_server:local"],
  "permissions": ["mcp.connect"],
  "mcpServers": [
    {
      "id": "local",
      "command": "mcp-server.sh",
      "transport": "stdio"
    }
  ],
  "defaultEnabled": true
}"#,
  )
  .expect("write mcp malformed plugin manifest");
  fs::write(
    source_root
      .join("commands")
      .join("mcp-malformed.inspect.json"),
    r#"{
  "title": "Inspect MCP Malformed Data",
  "description": "Return malformed Amentia MCP structured content.",
  "prompt": "Inspect MCP malformed data.",
  "execution": {
    "kind": "mcp.localInspect",
    "driver": "mcp",
    "entrypoint": "local.inspect"
  }
}"#,
  )
  .expect("write mcp malformed command manifest");
  fs::write(
    &server_path,
    r#"#!/bin/sh
cat >/dev/null
printf '{"jsonrpc":"2.0","id":1,"result":{}}\n'
cat <<'JSON'
{"jsonrpc":"2.0","id":2,"result":{"structuredContent":{"items":{"kind":"pluginResult"}}}}
JSON
"#,
  )
  .expect("write mcp malformed server");
  make_test_file_executable(&server_path, "mcp malformed server");
  replace_plugin_catalog(
    &mut context,
    vec![temp_manifest_plugin_entry(
      "mcp-malformed",
      "MCP Malformed",
      "MCP command plugin with malformed Amentia structured output",
      &["command:mcp-malformed.inspect", "mcp_server:local"],
      &["mcp.connect"],
      &plugin_manifest,
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
        "title": "MCP Malformed Thread"
      })),
    ),
  );

  let response = handle_request(
    &mut context,
    request(
      methods::PLUGIN_COMMAND_RUN,
      Some(json!({
        "threadId": "thread-1",
        "commandId": "mcp-malformed::mcp-malformed.inspect"
      })),
    ),
  );

  remove_temp_workspace(&workspace);
  remove_temp_plugin_source(&source_root);

  assert!(response.error.is_none());
  let result = response.result.expect("command run result");
  let items = result["items"].as_array().expect("items");
  assert_eq!(items[1]["kind"], "warning");
  assert_eq!(
    items[1]["attributes"]["pluginRunnerFailureKind"],
    "outputContract"
  );
  assert_eq!(items[1]["attributes"]["mcpProtocolStatus"], "completed");
  assert_eq!(
    items[1]["attributes"]["mcpStructuredContentStatus"],
    "amentiaOutputEnvelope"
  );
  assert_eq!(
    items[1]["attributes"]["pluginRunnerOutputStatus"],
    "malformedEnvelope"
  );
  assert_eq!(items[1]["attributes"]["pluginRunnerOutputParsed"], "false");
  assert!(items[1]["attributes"]["pluginRunnerOutputParseError"].is_string());
  assert!(items[1]["content"]
    .as_str()
    .expect("malformed structured content")
    .contains("malformed JSON output envelope"));
}

#[cfg(unix)]
#[test]
fn plugin_command_run_records_mcp_protocol_diagnostics() {
  let mut context = RuntimeContext::new_in_memory();
  let source_root = create_temp_plugin_bundle(
    "plugin-command-mcp-diagnostics",
    "mcp-diagnostics",
    "MCP Diagnostics",
  );
  let workspace = create_temp_workspace("plugin-command-mcp-diagnostics-workspace");
  let plugin_manifest = source_root.join("amentia-plugin.json");
  let server_path = source_root.join("mcp-server.sh");
  fs::write(
    &plugin_manifest,
    r#"{
  "name": "mcp-diagnostics",
  "version": "0.1.0",
  "displayName": "MCP Diagnostics",
  "description": "MCP protocol diagnostics plugin",
  "author": { "name": "Amentia" },
  "capabilities": ["command:mcp-diagnostics.run", "mcp_server:local"],
  "permissions": ["mcp.connect"],
  "mcpServers": [
    {
      "id": "local",
      "command": "mcp-server.sh",
      "transport": "stdio"
    }
  ],
  "defaultEnabled": true
}"#,
  )
  .expect("write mcp diagnostics plugin manifest");
  fs::write(
    source_root
      .join("commands")
      .join("mcp-diagnostics.run.json"),
    r#"{
  "title": "Run MCP Diagnostics",
  "description": "Run an MCP command that emits malformed stdout.",
  "prompt": "Run MCP diagnostics.",
  "execution": {
    "kind": "mcp.localDiagnostics",
    "driver": "mcp",
    "entrypoint": "local.inspect"
  }
}"#,
  )
  .expect("write mcp diagnostics command manifest");
  fs::write(
    &server_path,
    r#"#!/bin/sh
cat >/dev/null
printf 'debug line on stdout\n'
printf '{"jsonrpc":"2.0","id":1,"result":{}}\n'
"#,
  )
  .expect("write mcp diagnostics server");
  make_test_file_executable(&server_path, "mcp diagnostics server");
  replace_plugin_catalog(
    &mut context,
    vec![temp_manifest_plugin_entry(
      "mcp-diagnostics",
      "MCP Diagnostics",
      "MCP protocol diagnostics plugin",
      &["command:mcp-diagnostics.run", "mcp_server:local"],
      &["mcp.connect"],
      &plugin_manifest,
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
        "title": "MCP Diagnostics Thread"
      })),
    ),
  );

  let response = handle_request(
    &mut context,
    request(
      methods::PLUGIN_COMMAND_RUN,
      Some(json!({
        "threadId": "thread-1",
        "commandId": "mcp-diagnostics::mcp-diagnostics.run"
      })),
    ),
  );

  remove_temp_workspace(&workspace);
  remove_temp_plugin_source(&source_root);

  assert!(response.error.is_none());
  let result = response.result.expect("command run result");
  let items = result["items"].as_array().expect("items");
  assert_eq!(items[0]["kind"], "pluginCommand");
  assert_eq!(items[1]["kind"], "warning");
  assert_eq!(
    items[1]["attributes"]["pluginRunnerFailureKind"],
    "mcpProtocol"
  );
  assert_eq!(
    items[1]["attributes"]["pluginRunnerRecoveryHint"],
    "Keep MCP stdout reserved for JSON-RPC responses, move logs to stderr, and return the tools/call response with the expected request id."
  );
  assert_eq!(
    items[1]["attributes"]["mcpProtocolStatus"],
    "missingToolResponse"
  );
  assert_eq!(items[1]["attributes"]["mcpInitializeResponseSeen"], "true");
  assert_eq!(items[1]["attributes"]["mcpToolResponseSeen"], "false");
  assert_eq!(items[1]["attributes"]["mcpJsonResponseCount"], "1");
  assert_eq!(
    items[1]["attributes"]["pluginRunnerInputEnvelope"],
    "amentia.plugin.command.input"
  );
  assert_eq!(items[1]["attributes"]["pluginRunnerInputProvided"], "false");
  assert_eq!(items[1]["attributes"]["mcpInvalidJsonLineCount"], "1");
  assert_eq!(
    items[1]["attributes"]["mcpLastInvalidJsonPreview"],
    "debug line on stdout"
  );
  assert!(items[1]["content"]
    .as_str()
    .expect("warning content")
    .contains("initialized but did not return a tool response"));
}

#[test]
fn plugin_command_run_blocks_mcp_without_declared_mcp_permission() {
  let mut context = RuntimeContext::new_in_memory();
  let source_root = create_temp_plugin_bundle(
    "plugin-command-mcp-permission",
    "mcp-permission",
    "MCP Permission",
  );
  let workspace = create_temp_workspace("plugin-command-mcp-permission-workspace");
  let plugin_manifest = source_root.join("amentia-plugin.json");
  let server_path = source_root.join("mcp-server.sh");
  fs::write(
    &plugin_manifest,
    r#"{
  "name": "mcp-permission",
  "version": "0.1.0",
  "displayName": "MCP Permission",
  "description": "MCP command plugin missing permissions",
  "author": { "name": "Amentia" },
  "capabilities": ["command:mcp-permission.run", "mcp_server:local"],
  "permissions": [],
  "mcpServers": [
    {
      "id": "local",
      "command": "mcp-server.sh",
      "transport": "stdio"
    }
  ],
  "defaultEnabled": true
}"#,
  )
  .expect("write mcp permission plugin manifest");
  fs::write(
    source_root.join("commands").join("mcp-permission.run.json"),
    r#"{
  "title": "Run MCP",
  "description": "Run an MCP command.",
  "prompt": "Run the MCP command.",
  "execution": {
    "kind": "mcp.localRun",
    "driver": "mcp",
    "entrypoint": "local.run"
  }
}"#,
  )
  .expect("write mcp permission command manifest");
  fs::write(
    &server_path,
    r#"#!/bin/sh
printf '{"jsonrpc":"2.0","id":1,"result":{}}\n'
"#,
  )
  .expect("write mcp server");
  #[cfg(unix)]
  {
    make_test_file_executable(&server_path, "mcp server");
  }
  replace_plugin_catalog(
    &mut context,
    vec![temp_manifest_plugin_entry(
      "mcp-permission",
      "MCP Permission",
      "MCP command plugin missing permissions",
      &["command:mcp-permission.run", "mcp_server:local"],
      &[],
      &plugin_manifest,
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
        "title": "MCP Permission Thread"
      })),
    ),
  );

  let response = handle_request(
    &mut context,
    request(
      methods::PLUGIN_COMMAND_RUN,
      Some(json!({
        "threadId": "thread-1",
        "commandId": "mcp-permission::mcp-permission.run"
      })),
    ),
  );

  remove_temp_workspace(&workspace);
  remove_temp_plugin_source(&source_root);

  let error = response.error.expect("permission blocker error");
  assert_eq!(error.code, -32053);
  assert!(error.message.contains("mcp.connect"));
}

#[test]
fn plugin_command_run_blocks_connector_mcp_without_declared_network_permission() {
  let mut context = RuntimeContext::new_in_memory();
  let source_root =
    create_temp_plugin_bundle("plugin-command-mcp-network", "mcp-network", "MCP Network");
  let workspace = create_temp_workspace("plugin-command-mcp-network-workspace");
  let plugin_manifest = source_root.join("amentia-plugin.json");
  let server_path = source_root.join("mcp-server.sh");
  fs::write(
    &plugin_manifest,
    r#"{
  "name": "mcp-network",
  "version": "0.1.0",
  "displayName": "MCP Network",
  "description": "Connector MCP command plugin missing network permission",
  "author": { "name": "Amentia" },
  "capabilities": ["command:mcp-network.sync", "mcp_server:notion", "connector:notion"],
  "permissions": ["mcp.connect"],
  "mcpServers": [
    {
      "id": "notion",
      "command": "mcp-server.sh",
      "transport": "stdio"
    }
  ],
  "appConnectors": [
    {
      "id": "notion",
      "displayName": "Notion",
      "service": "notion",
      "homepage": "https://www.notion.so"
    }
  ],
  "authPolicy": {
    "type": "oauth2",
    "required": true,
    "scopes": ["insert_content"],
    "credentialStore": "local"
  },
  "defaultEnabled": true
}"#,
  )
  .expect("write mcp network plugin manifest");
  fs::write(
    source_root.join("commands").join("mcp-network.sync.json"),
    r#"{
  "title": "Sync MCP",
  "description": "Sync through a connector-backed MCP command.",
  "prompt": "Sync through MCP.",
  "execution": {
    "kind": "mcp.notionSync",
    "driver": "mcp",
    "entrypoint": "notion.sync"
  }
}"#,
  )
  .expect("write mcp network command manifest");
  fs::write(
    &server_path,
    r#"#!/bin/sh
printf '{"jsonrpc":"2.0","id":1,"result":{}}\n'
"#,
  )
  .expect("write mcp server");
  #[cfg(unix)]
  {
    make_test_file_executable(&server_path, "mcp server");
  }
  replace_plugin_catalog(
    &mut context,
    vec![temp_manifest_plugin_entry(
      "mcp-network",
      "MCP Network",
      "Connector MCP command plugin missing network permission",
      &[
        "command:mcp-network.sync",
        "mcp_server:notion",
        "connector:notion",
      ],
      &["mcp.connect"],
      &plugin_manifest,
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
        "title": "MCP Network Thread"
      })),
    ),
  );
  let authorize_response = handle_request(
    &mut context,
    request(
      methods::PLUGIN_CONNECTOR_AUTHORIZE,
      Some(json!({
        "connectorId": "mcp-network::notion"
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
        "commandId": "mcp-network::mcp-network.sync"
      })),
    ),
  );

  remove_temp_workspace(&workspace);
  remove_temp_plugin_source(&source_root);

  let error = response.error.expect("permission blocker error");
  assert_eq!(error.code, -32053);
  assert!(error.message.contains("network.outbound"));
}

#[cfg(unix)]
#[test]
fn plugin_command_run_records_stdio_runner_failure() {
  let mut context = RuntimeContext::new_in_memory();
  let source_root = create_temp_plugin_bundle(
    "plugin-command-stdio-failure",
    "stdio-failure",
    "Stdio Failure",
  );
  let workspace = create_temp_workspace("plugin-command-stdio-failure-workspace");
  let plugin_manifest = source_root.join("amentia-plugin.json");
  let runner_path = source_root.join("runner.sh");
  fs::write(
    source_root.join("commands").join("stdio-failure.run.json"),
    r#"{
  "title": "Run Failing Plugin",
  "description": "Execute a local stdio runner that fails.",
  "prompt": "Run the local plugin runner.",
  "execution": {
    "kind": "stdio.failure",
    "entrypoint": "runner.sh"
  }
}"#,
  )
  .expect("write command manifest");
  fs::write(
    &runner_path,
    r#"#!/bin/sh
cat >/dev/null
printf 'diagnostic stdout\n'
printf 'diagnostic stderr\n' >&2
exit 7
"#,
  )
  .expect("write runner");
  make_test_file_executable(&runner_path, "failure runner");
  replace_plugin_catalog(
    &mut context,
    vec![temp_manifest_plugin_entry(
      "stdio-failure",
      "Stdio Failure",
      "Failing stdio command plugin",
      &["command:stdio-failure.run"],
      &["file.read"],
      &plugin_manifest,
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
        "title": "Stdio Failure Thread"
      })),
    ),
  );

  let response = handle_request(
    &mut context,
    request(
      methods::PLUGIN_COMMAND_RUN,
      Some(json!({
        "threadId": "thread-1",
        "commandId": "stdio-failure::stdio-failure.run",
        "input": "Debug input"
      })),
    ),
  );

  remove_temp_workspace(&workspace);
  remove_temp_plugin_source(&source_root);

  assert!(response.error.is_none());
  let result = response.result.expect("command run result");
  let items = result["items"].as_array().expect("items");
  assert_eq!(items[1]["kind"], "warning");
  assert_eq!(items[1]["attributes"]["pluginCommandStatus"], "failed");
  assert_eq!(
    items[1]["attributes"]["pluginRunnerFailureKind"],
    "processExit"
  );
  assert_eq!(items[1]["attributes"]["pluginRunnerExitCode"], "7");
  assert_eq!(
    items[1]["attributes"]["pluginRunnerStderrPreview"],
    "diagnostic stderr"
  );
  assert_eq!(
    items[1]["attributes"]["pluginRunnerStdoutPreview"],
    "diagnostic stdout"
  );
  assert!(items[1]["content"]
    .as_str()
    .unwrap()
    .contains("Recovery Hint"));
  assert!(items[1]["content"]
    .as_str()
    .unwrap()
    .contains("Fix the runner error shown in stderr/stdout"));
  assert!(items[1]["content"]
    .as_str()
    .unwrap()
    .contains("diagnostic stderr"));
  assert!(items[1]["content"]
    .as_str()
    .unwrap()
    .contains("diagnostic stdout"));
  assert_eq!(
    context
      .execution_state
      .counts()
      .running_plugin_command_count(),
    0
  );
}

#[cfg(unix)]
#[test]
fn plugin_command_run_rejects_empty_stdio_output_envelope() {
  let mut context = RuntimeContext::new_in_memory();
  let source_root = create_temp_plugin_bundle(
    "plugin-command-empty-output",
    "empty-output",
    "Empty Output",
  );
  let workspace = create_temp_workspace("plugin-command-empty-output-workspace");
  let plugin_manifest = source_root.join("amentia-plugin.json");
  let runner_path = source_root.join("runner.sh");
  fs::write(
    source_root.join("commands").join("empty-output.run.json"),
    r#"{
  "title": "Run Empty Output Plugin",
  "description": "Execute a local stdio runner with an empty output envelope.",
  "prompt": "Run the local plugin runner.",
  "execution": {
    "kind": "stdio.emptyOutput",
    "entrypoint": "runner.sh"
  }
}"#,
  )
  .expect("write command manifest");
  fs::write(
    &runner_path,
    r#"#!/bin/sh
cat >/dev/null
printf '{}\n'
"#,
  )
  .expect("write runner");
  make_test_file_executable(&runner_path, "empty output runner");
  replace_plugin_catalog(
    &mut context,
    vec![temp_manifest_plugin_entry(
      "empty-output",
      "Empty Output",
      "Empty output command plugin",
      &["command:empty-output.run"],
      &["file.read"],
      &plugin_manifest,
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
        "title": "Empty Output Thread"
      })),
    ),
  );

  let response = handle_request(
    &mut context,
    request(
      methods::PLUGIN_COMMAND_RUN,
      Some(json!({
        "threadId": "thread-1",
        "commandId": "empty-output::empty-output.run"
      })),
    ),
  );

  remove_temp_workspace(&workspace);
  remove_temp_plugin_source(&source_root);

  assert!(response.error.is_none());
  let result = response.result.expect("command run result");
  let items = result["items"].as_array().expect("items");
  assert_eq!(items[1]["kind"], "warning");
  assert_eq!(items[1]["attributes"]["pluginCommandStatus"], "failed");
  assert_eq!(
    items[1]["attributes"]["pluginRunnerFailureKind"],
    "outputContract"
  );
  assert_eq!(
    items[1]["attributes"]["pluginRunnerOutputStatus"],
    "emptyEnvelope"
  );
  assert!(items[1]["content"]
    .as_str()
    .unwrap()
    .contains("without content, valid timeline items, or memory notes"));
}

#[cfg(unix)]
#[test]
fn plugin_command_run_rejects_malformed_stdio_output_envelope() {
  let mut context = RuntimeContext::new_in_memory();
  let source_root = create_temp_plugin_bundle(
    "plugin-command-malformed-output",
    "malformed-output",
    "Malformed Output",
  );
  let workspace = create_temp_workspace("plugin-command-malformed-output-workspace");
  let plugin_manifest = source_root.join("amentia-plugin.json");
  let runner_path = source_root.join("runner.sh");
  fs::write(
    source_root
      .join("commands")
      .join("malformed-output.run.json"),
    r#"{
  "title": "Run Malformed Output Plugin",
  "description": "Execute a local stdio runner with malformed JSON output.",
  "prompt": "Run the local plugin runner.",
  "execution": {
    "kind": "stdio.malformedOutput",
    "entrypoint": "runner.sh"
  }
}"#,
  )
  .expect("write command manifest");
  fs::write(
    &runner_path,
    r#"#!/bin/sh
cat >/dev/null
printf '{"content":'
"#,
  )
  .expect("write runner");
  make_test_file_executable(&runner_path, "malformed output runner");
  replace_plugin_catalog(
    &mut context,
    vec![temp_manifest_plugin_entry(
      "malformed-output",
      "Malformed Output",
      "Malformed output command plugin",
      &["command:malformed-output.run"],
      &["file.read"],
      &plugin_manifest,
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
        "title": "Malformed Output Thread"
      })),
    ),
  );

  let response = handle_request(
    &mut context,
    request(
      methods::PLUGIN_COMMAND_RUN,
      Some(json!({
        "threadId": "thread-1",
        "commandId": "malformed-output::malformed-output.run"
      })),
    ),
  );

  remove_temp_workspace(&workspace);
  remove_temp_plugin_source(&source_root);

  assert!(response.error.is_none());
  let result = response.result.expect("command run result");
  let items = result["items"].as_array().expect("items");
  assert_eq!(items[1]["kind"], "warning");
  assert_eq!(
    items[1]["attributes"]["pluginRunnerFailureKind"],
    "outputContract"
  );
  assert_eq!(
    items[1]["attributes"]["pluginRunnerOutputStatus"],
    "malformedEnvelope"
  );
  assert_eq!(items[1]["attributes"]["pluginRunnerOutputParsed"], "false");
  assert!(items[1]["attributes"]["pluginRunnerOutputParseError"].is_string());
  assert!(items[1]["content"]
    .as_str()
    .unwrap()
    .contains("malformed JSON output envelope"));
}

#[cfg(unix)]
#[test]
fn plugin_command_run_captures_stdio_runner_memory_notes() {
  let mut context = RuntimeContext::new_in_memory();
  let source_root = create_temp_plugin_bundle(
    "plugin-command-runner-memory",
    "runner-memory",
    "Runner Memory",
  );
  let workspace = create_temp_workspace("plugin-command-runner-memory-workspace");
  let plugin_manifest = source_root.join("amentia-plugin.json");
  let runner_path = source_root.join("runner.sh");
  fs::write(
    source_root.join("commands").join("runner-memory.run.json"),
    r#"{
  "title": "Run Memory Plugin",
  "description": "Execute a local stdio runner that emits memory notes.",
  "prompt": "Run the local plugin runner.",
  "execution": {
    "kind": "stdio.runnerMemory",
    "entrypoint": "runner.sh"
  }
}"#,
  )
  .expect("write command manifest");
  fs::write(
    &runner_path,
    r#"#!/bin/sh
cat >/dev/null
cat <<'JSON'
{
  "content": "Runner memory captured.",
  "memoryNotes": [
    {
      "title": "Runner Preference",
      "body": "Prefer narrow plugin output contracts.",
      "source": "plugin.runner-memory.custom",
      "tags": ["runner", "contract"]
    }
  ]
}
JSON
"#,
  )
  .expect("write runner");
  make_test_file_executable(&runner_path, "memory runner");
  replace_plugin_catalog(
    &mut context,
    vec![temp_manifest_plugin_entry(
      "runner-memory",
      "Runner Memory",
      "Runner memory command plugin",
      &["command:runner-memory.run"],
      &["file.read"],
      &plugin_manifest,
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
        "title": "Runner Memory Thread"
      })),
    ),
  );

  let response = handle_request(
    &mut context,
    request(
      methods::PLUGIN_COMMAND_RUN,
      Some(json!({
        "threadId": "thread-1",
        "commandId": "runner-memory::runner-memory.run"
      })),
    ),
  );

  remove_temp_workspace(&workspace);
  remove_temp_plugin_source(&source_root);

  assert!(response.error.is_none());
  let result = response.result.expect("command run result");
  let items = result["items"].as_array().expect("items");
  let memory_item = items
    .iter()
    .find(|item| item["title"] == "Plugin Memory Note Saved")
    .expect("runner memory item");
  assert_eq!(
    memory_item["attributes"]["memoryNoteTitle"],
    "Runner Preference"
  );
  let saved_note = context
    .memory_state
    .recent_notes(16)
    .into_iter()
    .find(|note| note.title == "Runner Preference")
    .expect("saved runner memory note");
  assert_eq!(saved_note.source, "plugin.runner-memory.custom");
  assert!(saved_note
    .body
    .contains("Prefer narrow plugin output contracts."));
  assert!(saved_note.tags.contains(&"contract".to_string()));
}

#[cfg(unix)]
#[test]
fn plugin_command_run_preflights_non_executable_runner() {
  let mut context = RuntimeContext::new_in_memory();
  let source_root =
    create_temp_plugin_bundle("plugin-command-non-executable", "non-exec", "Non Exec");
  let workspace = create_temp_workspace("plugin-command-non-executable-workspace");
  let plugin_manifest = source_root.join("amentia-plugin.json");
  let runner_path = source_root.join("runner.sh");
  fs::write(
    source_root.join("commands").join("non-exec.run.json"),
    r#"{
  "title": "Run Non Executable Plugin",
  "description": "Execute a local stdio runner without executable permissions.",
  "prompt": "Run the local plugin runner.",
  "execution": {
    "kind": "stdio.nonExecutable",
    "entrypoint": "runner.sh"
  }
}"#,
  )
  .expect("write command manifest");
  fs::write(
    &runner_path,
    r#"#!/bin/sh
printf 'should not run\n'
"#,
  )
  .expect("write runner");
  replace_plugin_catalog(
    &mut context,
    vec![temp_manifest_plugin_entry(
      "non-exec",
      "Non Exec",
      "Non executable stdio command plugin",
      &["command:non-exec.run"],
      &["file.read"],
      &plugin_manifest,
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
        "title": "Non Exec Thread"
      })),
    ),
  );

  let response = handle_request(
    &mut context,
    request(
      methods::PLUGIN_COMMAND_RUN,
      Some(json!({
        "threadId": "thread-1",
        "commandId": "non-exec::non-exec.run"
      })),
    ),
  );

  remove_temp_workspace(&workspace);
  remove_temp_plugin_source(&source_root);

  let error = response.error.expect("runner setup error");
  assert_eq!(error.code, -32053);
  assert!(error.message.contains("not executable"));
  assert_eq!(
    context
      .execution_state
      .counts()
      .running_plugin_command_count(),
    0
  );
}

#[cfg(unix)]
#[test]
fn plugin_command_run_accepts_runner_timeline_items() {
  let mut context = RuntimeContext::new_in_memory();
  let source_root =
    create_temp_plugin_bundle("plugin-command-owned-items", "owned-items", "Owned Items");
  let workspace = create_temp_workspace("plugin-command-owned-items-workspace");
  let plugin_manifest = source_root.join("amentia-plugin.json");
  let runner_path = source_root.join("runner.sh");
  fs::write(
    source_root.join("commands").join("owned-items.run.json"),
    r#"{
  "title": "Run Owned Items",
  "description": "Return timeline items from the plugin runner.",
  "prompt": "Run the local plugin runner.",
  "execution": {
    "kind": "stdio.ownedItems",
    "entrypoint": "runner.sh"
  }
}"#,
  )
  .expect("write command manifest");
  fs::write(
    &runner_path,
    r#"#!/bin/sh
cat >/dev/null
cat <<'JSON'
{
  "items": [
    {
      "kind": "pluginResult",
      "title": "Runner Item",
      "content": "Owned timeline item.",
      "attributes": { "runner": "stdio" }
    }
  ]
}
JSON
"#,
  )
  .expect("write runner");
  make_test_file_executable(&runner_path, "owned items runner");
  replace_plugin_catalog(
    &mut context,
    vec![temp_manifest_plugin_entry(
      "owned-items",
      "Owned Items",
      "Owned item plugin",
      &["command:owned-items.run"],
      &["file.read"],
      &plugin_manifest,
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
        "title": "Owned Items Thread"
      })),
    ),
  );

  let response = handle_request(
    &mut context,
    request(
      methods::PLUGIN_COMMAND_RUN,
      Some(json!({
        "threadId": "thread-1",
        "commandId": "owned-items::owned-items.run"
      })),
    ),
  );

  remove_temp_workspace(&workspace);
  remove_temp_plugin_source(&source_root);

  assert!(response.error.is_none());
  let result = response.result.expect("command run result");
  let items = result["items"].as_array().expect("items");
  assert_eq!(items.len(), 2);
  assert_eq!(items[1]["title"], "Runner Item");
  assert_eq!(items[1]["content"], "Owned timeline item.");
  assert_eq!(items[1]["attributes"]["runner"], "stdio");
  assert_eq!(items[1]["attributes"]["pluginId"], "owned-items");
  assert_eq!(items[1]["attributes"]["executionKind"], "stdio.ownedItems");
}

#[test]
fn plugin_command_run_rejects_runner_entrypoint_escape() {
  let mut context = RuntimeContext::new_in_memory();
  let source_root = create_temp_plugin_bundle(
    "plugin-command-entrypoint-escape",
    "escape-runner",
    "Escape Runner",
  );
  let workspace = create_temp_workspace("plugin-command-entrypoint-escape-workspace");
  let plugin_manifest = source_root.join("amentia-plugin.json");
  fs::write(
    source_root.join("commands").join("escape-runner.run.json"),
    r#"{
  "title": "Run Escape Runner",
  "description": "Attempt to escape the plugin bundle.",
  "prompt": "Run the local plugin runner.",
  "execution": {
    "kind": "stdio.escape",
    "entrypoint": "../runner.sh"
  }
}"#,
  )
  .expect("write command manifest");
  replace_plugin_catalog(
    &mut context,
    vec![temp_manifest_plugin_entry(
      "escape-runner",
      "Escape Runner",
      "Escape runner plugin",
      &["command:escape-runner.run"],
      &["file.read"],
      &plugin_manifest,
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
        "title": "Escape Runner Thread"
      })),
    ),
  );

  let response = handle_request(
    &mut context,
    request(
      methods::PLUGIN_COMMAND_RUN,
      Some(json!({
        "threadId": "thread-1",
        "commandId": "escape-runner::escape-runner.run"
      })),
    ),
  );

  remove_temp_workspace(&workspace);
  remove_temp_plugin_source(&source_root);

  let error = response.error.expect("runner setup error");
  assert_eq!(error.code, -32053);
  assert!(error.message.contains("inside the plugin bundle"));
  assert_eq!(
    context
      .execution_state
      .counts()
      .running_plugin_command_count(),
    0
  );
}
