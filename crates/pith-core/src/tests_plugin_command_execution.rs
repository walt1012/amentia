use super::test_support::{
  bundled_manifest_plugin_entry, create_temp_plugin_bundle, create_temp_workspace,
  replace_plugin_catalog, request,
};
use super::*;
use pith_plugin_host::PluginCatalogEntry;
use pith_protocol::methods;
use serde_json::json;
use std::fs;

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
      &[
        "command:workspace.capture-note",
        "prompt_pack:workspace.notes",
      ],
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

  fs::remove_dir_all(&workspace).expect("cleanup temp workspace");

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
fn bundled_builtin_plugin_commands_return_owned_results() {
  let mut context = RuntimeContext::new_in_memory();
  let workspace = create_temp_workspace("bundled-plugin-results");
  fs::write(workspace.join("README.md"), "# Bundled Plugin Results\n").expect("write readme");
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

  fs::remove_dir_all(&workspace).expect("cleanup temp workspace");

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
    create_temp_plugin_bundle("plugin-command-contract", "prompt-only", "Prompt Only");
  let workspace = create_temp_workspace("plugin-command-contract-workspace");
  let plugin_manifest = source_root.join("pith-plugin.json");
  replace_plugin_catalog(
    &mut context,
    vec![PluginCatalogEntry {
      id: "prompt-only".to_string(),
      name: "prompt-only".to_string(),
      version: "0.1.0".to_string(),
      display_name: "Prompt Only".to_string(),
      status: "ready".to_string(),
      description: "Prompt-only command plugin".to_string(),
      author_name: Some("Pith".to_string()),
      enabled: true,
      default_enabled: true,
      capabilities: vec!["command:prompt-only.run".to_string()],
      permissions: vec!["file.read".to_string()],
      manifest_path: plugin_manifest.display().to_string(),
      provenance: "test".to_string(),
      validation_error: None,
      validation_hint: None,
    }],
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
        "commandId": "prompt-only::prompt-only.run"
      })),
    ),
  );

  fs::remove_dir_all(&workspace).expect("cleanup temp workspace");
  fs::remove_dir_all(source_root.parent().expect("plugin root")).expect("cleanup plugin source");

  let error = response.error.expect("command contract error");
  assert_eq!(error.code, -32053);
  assert!(error
    .message
    .contains("requires an explicit execution contract"));
}
