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

#[cfg(unix)]
#[test]
fn plugin_command_run_executes_bounded_stdio_runner() {
  use std::os::unix::fs::PermissionsExt;

  let mut context = RuntimeContext::new_in_memory();
  let source_root = create_temp_plugin_bundle(
    "plugin-command-stdio-runner",
    "stdio-runner",
    "Stdio Runner",
  );
  let workspace = create_temp_workspace("plugin-command-stdio-workspace");
  let plugin_manifest = source_root.join("pith-plugin.json");
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
[ -n "$PITH_PLUGIN_SANDBOX_TEMP" ] || exit 9
printf '{"content":"External runner completed."}\n'
"#,
  )
  .expect("write runner");
  let mut permissions = fs::metadata(&runner_path)
    .expect("runner metadata")
    .permissions();
  permissions.set_mode(0o755);
  fs::set_permissions(&runner_path, permissions).expect("set runner permissions");
  replace_plugin_catalog(
    &mut context,
    vec![PluginCatalogEntry {
      id: "stdio-runner".to_string(),
      name: "stdio-runner".to_string(),
      version: "0.1.0".to_string(),
      display_name: "Stdio Runner".to_string(),
      status: "ready".to_string(),
      description: "Stdio command plugin".to_string(),
      author_name: Some("Pith".to_string()),
      enabled: true,
      default_enabled: true,
      capabilities: vec!["command:stdio-runner.run".to_string()],
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

  fs::remove_dir_all(&workspace).expect("cleanup temp workspace");
  fs::remove_dir_all(source_root.parent().expect("plugin root")).expect("cleanup plugin source");

  assert!(response.error.is_none());
  let result = response.result.expect("command run result");
  let items = result["items"].as_array().expect("items");
  assert_eq!(items[1]["kind"], "pluginResult");
  assert_eq!(items[1]["attributes"]["executionKind"], "stdio.echo");
  assert!(items[1]["attributes"]["sandboxBackend"].is_string());
  assert!(items[1]["attributes"]["sandboxTemporaryRoot"].is_string());
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
fn plugin_command_run_accepts_runner_timeline_items() {
  use std::os::unix::fs::PermissionsExt;

  let mut context = RuntimeContext::new_in_memory();
  let source_root =
    create_temp_plugin_bundle("plugin-command-owned-items", "owned-items", "Owned Items");
  let workspace = create_temp_workspace("plugin-command-owned-items-workspace");
  let plugin_manifest = source_root.join("pith-plugin.json");
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
  let mut permissions = fs::metadata(&runner_path)
    .expect("runner metadata")
    .permissions();
  permissions.set_mode(0o755);
  fs::set_permissions(&runner_path, permissions).expect("set runner permissions");
  replace_plugin_catalog(
    &mut context,
    vec![PluginCatalogEntry {
      id: "owned-items".to_string(),
      name: "owned-items".to_string(),
      version: "0.1.0".to_string(),
      display_name: "Owned Items".to_string(),
      status: "ready".to_string(),
      description: "Owned item plugin".to_string(),
      author_name: Some("Pith".to_string()),
      enabled: true,
      default_enabled: true,
      capabilities: vec!["command:owned-items.run".to_string()],
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

  fs::remove_dir_all(&workspace).expect("cleanup temp workspace");
  fs::remove_dir_all(source_root.parent().expect("plugin root")).expect("cleanup plugin source");

  assert!(response.error.is_none());
  let result = response.result.expect("command run result");
  let items = result["items"].as_array().expect("items");
  assert_eq!(items.len(), 2);
  assert_eq!(items[1]["title"], "Runner Item");
  assert_eq!(items[1]["content"], "Owned timeline item.");
  assert_eq!(items[1]["attributes"]["runner"], "stdio");
  assert_eq!(items[1]["attributes"]["pluginId"], "owned-items");
  assert!(items[1]["attributes"]["sandboxBackend"].is_string());
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
  let plugin_manifest = source_root.join("pith-plugin.json");
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
    vec![PluginCatalogEntry {
      id: "escape-runner".to_string(),
      name: "escape-runner".to_string(),
      version: "0.1.0".to_string(),
      display_name: "Escape Runner".to_string(),
      status: "ready".to_string(),
      description: "Escape runner plugin".to_string(),
      author_name: Some("Pith".to_string()),
      enabled: true,
      default_enabled: true,
      capabilities: vec!["command:escape-runner.run".to_string()],
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

  fs::remove_dir_all(&workspace).expect("cleanup temp workspace");
  fs::remove_dir_all(source_root.parent().expect("plugin root")).expect("cleanup plugin source");

  let error = response.error.expect("entrypoint escape error");
  assert_eq!(error.code, -32054);
  assert!(error.message.contains("inside the plugin bundle"));
  assert_eq!(
    context
      .execution_state
      .counts()
      .running_plugin_command_count(),
    0
  );
}
