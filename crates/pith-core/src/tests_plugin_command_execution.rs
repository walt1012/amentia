use super::test_support::{
  bundled_manifest_plugin_entry, create_temp_plugin_bundle, create_temp_workspace,
  replace_plugin_catalog, request,
};
use super::*;
use pith_plugin_host::PluginCatalogEntry;
use pith_protocol::methods;
use serde_json::json;
use std::fs;

#[cfg(unix)]
fn create_notion_stdio_runner_plugin(label: &str) -> (std::path::PathBuf, PluginCatalogEntry) {
  use std::os::unix::fs::PermissionsExt;

  let source_root = create_temp_plugin_bundle(label, "notion-runner", "Notion Runner");
  let plugin_manifest = source_root.join("pith-plugin.json");
  let runner_path = source_root.join("runner.sh");
  fs::write(
    &plugin_manifest,
    r#"{
  "name": "notion-runner",
  "version": "0.1.0",
  "displayName": "Notion Runner",
  "description": "Connector-backed stdio command plugin",
  "author": { "name": "Pith" },
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
    "credentialStore": "keychain"
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
case "$payload" in *'"provider":"pith.localCredentialProvider"'*) provider=true;; *) provider=false;; esac
case "$payload" in *'"handle":"notion-runner::notion"'*) handle=true;; *) handle=false;; esac
case "$payload" in *'"store":"keychain"'*) store=true;; *) store=false;; esac
case "$payload" in *'"label":"Notion authorization marker"'*) label=true;; *) label=false;; esac
case "$payload" in *"access_token"*|*"refresh_token"*|*"secret"*) secret_leak=true;; *) secret_leak=false;; esac
printf '{"content":"connectorId=%s provider=%s handle=%s store=%s label=%s secretLeak=%s","memoryNotes":[{"title":"Approved Connector Memory","body":"Connector runner memory survives approval execution.","source":"plugin.notion-runner.approved","tags":["connector","approved"]}]}\n' "$connector_id" "$provider" "$handle" "$store" "$label" "$secret_leak"
"#,
  )
  .expect("write connector runner");
  let mut permissions = fs::metadata(&runner_path)
    .expect("runner metadata")
    .permissions();
  permissions.set_mode(0o755);
  fs::set_permissions(&runner_path, permissions).expect("set runner permissions");
  let catalog_entry = PluginCatalogEntry {
    id: "notion-runner".to_string(),
    name: "notion-runner".to_string(),
    version: "0.1.0".to_string(),
    display_name: "Notion Runner".to_string(),
    status: "ready".to_string(),
    description: "Connector-backed stdio command plugin".to_string(),
    author_name: Some("Pith".to_string()),
    enabled: true,
    default_enabled: true,
    capabilities: vec![
      "command:notion-runner.sync".to_string(),
      "connector:notion".to_string(),
    ],
    permissions: vec!["network.outbound".to_string(), "mcp.connect".to_string()],
    manifest_path: plugin_manifest.display().to_string(),
    provenance: "test".to_string(),
    validation_error: None,
    validation_hint: None,
  };
  (source_root, catalog_entry)
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
  let data = error.data.expect("command readiness error data");
  assert_eq!(data["pluginId"], "prompt-only");
  assert_eq!(data["commandId"], "prompt-only::prompt-only.run");
  assert_eq!(data["runStatus"], "missingExecution");
  assert!(data["runRepairHint"]
    .as_str()
    .expect("repair hint")
    .contains("execution contract"));
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
  assert_eq!(items[1]["attributes"]["sandboxMode"], "workspaceReadWrite");
  assert!(items[1]["attributes"]["sandboxBackend"].is_string());
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
  use std::os::unix::fs::PermissionsExt;

  let mut context = RuntimeContext::new_in_memory();
  let source_root = create_temp_plugin_bundle(
    "plugin-command-required-input",
    "required-input",
    "Required Input",
  );
  let workspace = create_temp_workspace("plugin-command-required-input-workspace");
  let plugin_manifest = source_root.join("pith-plugin.json");
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
      "envelope": "pith.plugin.command.input",
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
  let mut permissions = fs::metadata(&runner_path)
    .expect("runner metadata")
    .permissions();
  permissions.set_mode(0o755);
  fs::set_permissions(&runner_path, permissions).expect("set runner permissions");
  replace_plugin_catalog(
    &mut context,
    vec![PluginCatalogEntry {
      id: "required-input".to_string(),
      name: "required-input".to_string(),
      version: "0.1.0".to_string(),
      display_name: "Required Input".to_string(),
      status: "ready".to_string(),
      description: "Required input command plugin".to_string(),
      author_name: Some("Pith".to_string()),
      enabled: true,
      default_enabled: true,
      capabilities: vec!["command:required-input.run".to_string()],
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

  fs::remove_dir_all(&workspace).expect("cleanup temp workspace");
  fs::remove_dir_all(source_root.parent().expect("plugin root")).expect("cleanup plugin source");

  let error = response.error.expect("required input error");
  assert_eq!(error.code, -32053);
  assert!(error
    .message
    .contains("requires command input field `input`"));
  let data = error.data.expect("input contract error data");
  assert_eq!(data["pluginId"], "required-input");
  assert_eq!(data["commandId"], "required-input::required-input.run");
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
  use std::os::unix::fs::PermissionsExt;

  let mut context = RuntimeContext::new_in_memory();
  let source_root = create_temp_plugin_bundle(
    "plugin-command-empty-connector-scope",
    "notion-runner",
    "Notion Runner",
  );
  let workspace = create_temp_workspace("plugin-command-empty-connector-scope-workspace");
  let plugin_manifest = source_root.join("pith-plugin.json");
  let runner_path = source_root.join("runner.sh");
  fs::write(
    &plugin_manifest,
    r#"{
  "name": "notion-runner",
  "version": "0.1.0",
  "displayName": "Notion Runner",
  "description": "Connector plugin with a local status command",
  "author": { "name": "Pith" },
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
    "credentialStore": "keychain"
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
  let mut permissions = fs::metadata(&runner_path)
    .expect("runner metadata")
    .permissions();
  permissions.set_mode(0o755);
  fs::set_permissions(&runner_path, permissions).expect("set runner permissions");
  replace_plugin_catalog(
    &mut context,
    vec![PluginCatalogEntry {
      id: "notion-runner".to_string(),
      name: "notion-runner".to_string(),
      version: "0.1.0".to_string(),
      display_name: "Notion Runner".to_string(),
      status: "ready".to_string(),
      description: "Connector plugin with a local status command".to_string(),
      author_name: Some("Pith".to_string()),
      enabled: true,
      default_enabled: true,
      capabilities: vec![
        "command:notion-runner.status".to_string(),
        "connector:notion".to_string(),
      ],
      permissions: vec!["network.outbound".to_string()],
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

  fs::remove_dir_all(&workspace).expect("cleanup temp workspace");
  fs::remove_dir_all(source_root.parent().expect("plugin root")).expect("cleanup plugin source");

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
  use std::os::unix::fs::PermissionsExt;

  let mut context = RuntimeContext::new_in_memory();
  let source_root = create_temp_plugin_bundle(
    "plugin-command-auth-free-connector",
    "browser-runner",
    "Browser Runner",
  );
  let workspace = create_temp_workspace("plugin-command-auth-free-connector-workspace");
  let plugin_manifest = source_root.join("pith-plugin.json");
  let runner_path = source_root.join("runner.sh");
  fs::write(
    &plugin_manifest,
    r#"{
  "name": "browser-runner",
  "version": "0.1.0",
  "displayName": "Browser Runner",
  "description": "Auth-free connector command plugin",
  "author": { "name": "Pith" },
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
case "$payload" in *'"provider":"pith.noCredentialRequired"'*) provider=true;; *) provider=false;; esac
case "$payload" in *'"handle":"browser-runner::web"'*) handle=true;; *) handle=false;; esac
case "$payload" in *'"store":"none"'*) store=true;; *) store=false;; esac
case "$payload" in *'"envKey"'*) env_key=true;; *) env_key=false;; esac
case "$payload" in *"access_token"*|*"refresh_token"*|*"secret"*) secret_leak=true;; *) secret_leak=false;; esac
printf '{"content":"connectorId=%s provider=%s handle=%s store=%s envKey=%s secretLeak=%s"}\n' "$connector_id" "$provider" "$handle" "$store" "$env_key" "$secret_leak"
"#,
  )
  .expect("write auth-free connector runner");
  let mut permissions = fs::metadata(&runner_path)
    .expect("runner metadata")
    .permissions();
  permissions.set_mode(0o755);
  fs::set_permissions(&runner_path, permissions).expect("set runner permissions");
  replace_plugin_catalog(
    &mut context,
    vec![PluginCatalogEntry {
      id: "browser-runner".to_string(),
      name: "browser-runner".to_string(),
      version: "0.1.0".to_string(),
      display_name: "Browser Runner".to_string(),
      status: "ready".to_string(),
      description: "Auth-free connector command plugin".to_string(),
      author_name: Some("Pith".to_string()),
      enabled: true,
      default_enabled: true,
      capabilities: vec![
        "command:browser-runner.search".to_string(),
        "connector:web".to_string(),
      ],
      permissions: vec!["network.outbound".to_string()],
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

  fs::remove_dir_all(&workspace).expect("cleanup temp workspace");
  fs::remove_dir_all(source_root.parent().expect("plugin root")).expect("cleanup plugin source");

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
    "pith.noCredentialRequired"
  );
  assert_eq!(items[0]["attributes"]["connectorSecretBindings"], "none");
  assert_eq!(items[1]["kind"], "pluginResult");
  assert_eq!(items[1]["attributes"]["pluginRunnerConnectorCount"], "1");
  assert_eq!(
    items[1]["attributes"]["pluginRunnerConnectorIds"],
    "browser-runner::web"
  );
  assert_eq!(
    items[1]["attributes"]["pluginRunnerCredentialProviders"],
    "pith.noCredentialRequired"
  );
  assert_eq!(items[1]["attributes"]["pluginRunnerSecretBindings"], "none");
  assert_eq!(items[1]["attributes"]["sandboxNetworkAllowed"], "true");
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
  assert_eq!(items[0]["attributes"]["connectorServices"], "notion");
  assert_eq!(
    items[0]["attributes"]["connectorCredentialProviders"],
    "pith.localCredentialProvider"
  );
  assert_eq!(
    items[0]["attributes"]["connectorCredentialHandles"],
    "notion-runner::notion"
  );
  assert_eq!(
    items[0]["attributes"]["connectorCredentialLabels"],
    "Notion authorization marker"
  );
  assert_eq!(
    items[0]["attributes"]["connectorSecretBindings"],
    "marker-only"
  );
  assert!(items[0]["attributes"]["connectorCredentialAuthorizedAt"].is_string());
  assert_eq!(items[1]["kind"], "approvalRequested");
  assert_eq!(items[1]["attributes"]["connectorServices"], "notion");
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

  fs::remove_dir_all(&workspace).expect("cleanup temp workspace");
  fs::remove_dir_all(source_root.parent().expect("plugin root")).expect("cleanup plugin source");

  assert!(approval_response.error.is_none());
  let approval_result = approval_response.result.expect("approval result");
  let items = approval_result["items"].as_array().expect("approval items");
  assert_eq!(items[0]["kind"], "approvalResolved");
  assert_eq!(items[1]["kind"], "pluginCommand");
  assert_eq!(
    items[1]["attributes"]["connectorIds"],
    "notion-runner::notion"
  );
  assert_eq!(items[1]["attributes"]["connectorServices"], "notion");
  assert_eq!(
    items[1]["attributes"]["connectorCredentialProviders"],
    "pith.localCredentialProvider"
  );
  assert_eq!(
    items[1]["attributes"]["connectorCredentialLabels"],
    "Notion authorization marker"
  );
  assert_eq!(
    items[1]["attributes"]["connectorSecretBindings"],
    "marker-only"
  );
  assert_eq!(items[2]["kind"], "pluginResult");
  assert_eq!(items[2]["attributes"]["pluginRunnerConnectorCount"], "1");
  assert_eq!(
    items[2]["attributes"]["pluginRunnerConnectorIds"],
    "notion-runner::notion"
  );
  assert_eq!(
    items[2]["attributes"]["pluginRunnerConnectorServices"],
    "notion"
  );
  assert_eq!(
    items[2]["attributes"]["pluginRunnerConnectorStores"],
    "keychain"
  );
  assert_eq!(
    items[2]["attributes"]["pluginRunnerCredentialProviders"],
    "pith.localCredentialProvider"
  );
  assert_eq!(
    items[2]["attributes"]["pluginRunnerCredentialHandles"],
    "notion-runner::notion"
  );
  assert_eq!(
    items[2]["attributes"]["pluginRunnerExecutionDriver"],
    "stdio"
  );
  assert_eq!(
    items[2]["attributes"]["pluginRunnerExecutionKind"],
    "stdio.notionSync"
  );
  assert_eq!(
    items[2]["attributes"]["pluginRunnerEntrypoint"],
    "runner.sh"
  );
  assert_eq!(
    items[2]["attributes"]["pluginRunnerEntrypointCheck"],
    "ready"
  );
  assert_eq!(
    items[2]["attributes"]["pluginRunnerEntrypointFileKind"],
    "file"
  );
  assert_eq!(
    items[2]["attributes"]["pluginRunnerEntrypointExecutable"],
    "true"
  );
  assert!(items[2]["attributes"]["pluginRunnerPluginRoot"].is_string());
  assert!(items[2]["attributes"]["pluginRunnerResolvedEntrypoint"].is_string());
  assert_eq!(
    items[2]["attributes"]["pluginRunnerCredentialLabels"],
    "Notion authorization marker"
  );
  assert_eq!(
    items[2]["attributes"]["pluginRunnerSecretBindings"],
    "marker-only"
  );
  assert!(items[2]["attributes"]["pluginRunnerCredentialAuthorizedAt"].is_string());
  assert_eq!(items[2]["attributes"]["sandboxNetworkAllowed"], "true");
  assert!(items[2]["attributes"]["sandboxNetworkPolicy"]
    .as_str()
    .expect("sandbox network policy")
    .contains("network allowed"));
  assert_eq!(
    items[2]["content"],
    "connectorId=true provider=true handle=true store=true label=true secretLeak=false"
  );
  assert_eq!(
    items[2]["attributes"]["pluginRunnerOutputMemoryNoteCount"],
    "1"
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

  fs::remove_dir_all(&workspace).expect("cleanup temp workspace");
  fs::remove_dir_all(source_root.parent().expect("plugin root")).expect("cleanup plugin source");

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
  assert!(data["runRepairHint"]
    .as_str()
    .expect("repair hint")
    .contains("required permission"));
}

#[cfg(unix)]
#[test]
fn plugin_command_run_executes_mcp_stdio_connector_action() {
  use std::os::unix::fs::PermissionsExt;

  let mut context = RuntimeContext::new_in_memory();
  let source_root =
    create_temp_plugin_bundle("plugin-command-mcp-runner", "notion-mcp", "Notion MCP");
  let workspace = create_temp_workspace("plugin-command-mcp-workspace");
  let plugin_manifest = source_root.join("pith-plugin.json");
  let server_path = source_root.join("mcp-server.sh");
  fs::write(
    &plugin_manifest,
    r#"{
  "name": "notion-mcp",
  "version": "0.1.0",
  "displayName": "Notion MCP",
  "description": "Connector-backed MCP command plugin",
  "author": { "name": "Pith" },
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
    "credentialStore": "keychain"
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
case "$payload" in *'"provider":"pith.localCredentialProvider"'*) provider=true;; *) provider=false;; esac
case "$payload" in *'"handle":"notion-mcp::notion"'*) handle=true;; *) handle=false;; esac
case "$payload" in *"access_token"*|*"refresh_token"*|*"secret"*) secret_leak=true;; *) secret_leak=false;; esac
if [ "$PITH_PLUGIN_CREDENTIAL_NOTION_MCP__NOTION" = "notion-local-token" ]; then credential_env=true; else credential_env=false; fi
case "$payload" in *"notion-local-token"*) token_leak=true;; *) token_leak=false;; esac
printf '{"jsonrpc":"2.0","id":1,"result":{}}\n'
printf '{"jsonrpc":"2.0","id":2,"result":{"content":[{"type":"text","text":"method=%s tool=%s provider=%s handle=%s secretLeak=%s credentialEnv=%s tokenLeak=%s"}]}}\n' "$method" "$tool" "$provider" "$handle" "$secret_leak" "$credential_env" "$token_leak"
"#,
  )
  .expect("write mcp server");
  let mut permissions = fs::metadata(&server_path)
    .expect("mcp server metadata")
    .permissions();
  permissions.set_mode(0o755);
  fs::set_permissions(&server_path, permissions).expect("set mcp server permissions");
  replace_plugin_catalog(
    &mut context,
    vec![PluginCatalogEntry {
      id: "notion-mcp".to_string(),
      name: "notion-mcp".to_string(),
      version: "0.1.0".to_string(),
      display_name: "Notion MCP".to_string(),
      status: "ready".to_string(),
      description: "Connector-backed MCP command plugin".to_string(),
      author_name: Some("Pith".to_string()),
      enabled: true,
      default_enabled: true,
      capabilities: vec![
        "command:notion-mcp.create-task".to_string(),
        "mcp_server:notion".to_string(),
        "connector:notion".to_string(),
      ],
      permissions: vec!["network.outbound".to_string(), "mcp.connect".to_string()],
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
  assert_eq!(items[1]["title"], "Plugin Approval Requested");
  assert_eq!(
    items[1]["attributes"]["pluginCommandRunId"],
    items[0]["attributes"]["pluginCommandRunId"]
  );
  assert_eq!(items[1]["attributes"]["connectorServices"], "notion");
  assert_eq!(
    items[1]["attributes"]["connectorCredentialProviders"],
    "pith.localCredentialProvider"
  );
  assert_eq!(
    items[1]["attributes"]["connectorCredentialHandles"],
    "notion-mcp::notion"
  );
  assert_eq!(
    items[1]["attributes"]["connectorSecretBindings"],
    "env-bound"
  );
  assert!(items[1]["content"]
    .as_str()
    .expect("approval content")
    .contains("secrets env-bound"));
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

  fs::remove_dir_all(&workspace).expect("cleanup temp workspace");
  fs::remove_dir_all(source_root.parent().expect("plugin root")).expect("cleanup plugin source");

  assert!(approval_response.error.is_none());
  let approval_result = approval_response.result.expect("approval result");
  let items = approval_result["items"].as_array().expect("approval items");
  assert_eq!(items[0]["kind"], "approvalResolved");
  assert_eq!(items[1]["kind"], "pluginCommand");
  assert_eq!(items[2]["kind"], "pluginResult");
  assert_eq!(
    items[1]["attributes"]["pluginCommandRunId"],
    "thread-1::notion-mcp::notion-mcp.create-task"
  );
  assert_eq!(
    items[2]["attributes"]["pluginCommandRunId"],
    items[1]["attributes"]["pluginCommandRunId"]
  );
  assert_eq!(
    items[2]["attributes"]["executionKind"],
    "mcp.notionCreateTask"
  );
  assert_eq!(items[2]["attributes"]["mcpServerId"], "notion");
  assert_eq!(items[2]["attributes"]["mcpToolName"], "createTask");
  assert_eq!(items[2]["attributes"]["pluginRunnerExecutionDriver"], "mcp");
  assert_eq!(
    items[2]["attributes"]["pluginRunnerExecutionKind"],
    "mcp.notionCreateTask"
  );
  assert_eq!(
    items[2]["attributes"]["pluginRunnerEntrypoint"],
    "notion.createTask"
  );
  assert_eq!(items[2]["attributes"]["mcpServerCommand"], "mcp-server.sh");
  assert_eq!(
    items[2]["attributes"]["pluginRunnerEntrypointCheck"],
    "ready"
  );
  assert_eq!(
    items[2]["attributes"]["pluginRunnerEntrypointFileKind"],
    "file"
  );
  assert_eq!(
    items[2]["attributes"]["pluginRunnerEntrypointExecutable"],
    "true"
  );
  assert!(items[2]["attributes"]["pluginRunnerPluginRoot"].is_string());
  assert!(items[2]["attributes"]["pluginRunnerResolvedEntrypoint"].is_string());
  assert_eq!(items[2]["attributes"]["mcpProtocolStatus"], "completed");
  assert_eq!(items[2]["attributes"]["mcpInitializeResponseSeen"], "true");
  assert_eq!(items[2]["attributes"]["mcpToolResponseSeen"], "true");
  assert_eq!(items[2]["attributes"]["pluginRunnerConnectorCount"], "1");
  assert_eq!(
    items[2]["attributes"]["pluginRunnerCredentialProviders"],
    "pith.localCredentialProvider"
  );
  assert_eq!(
    items[2]["attributes"]["pluginRunnerCredentialLabels"],
    "Notion authorization marker"
  );
  assert_eq!(
    items[2]["attributes"]["pluginRunnerSecretBindings"],
    "env-bound"
  );
  assert!(items[2]["attributes"]["pluginRunnerCredentialAuthorizedAt"].is_string());
  assert_eq!(
    items[2]["content"],
    "method=true tool=true provider=true handle=true secretLeak=false credentialEnv=true tokenLeak=false"
  );
}

#[cfg(unix)]
#[test]
fn plugin_command_run_accepts_mcp_structured_pith_output() {
  use std::os::unix::fs::PermissionsExt;

  let mut context = RuntimeContext::new_in_memory();
  let source_root = create_temp_plugin_bundle(
    "plugin-command-mcp-structured-output",
    "mcp-structured",
    "MCP Structured",
  );
  let workspace = create_temp_workspace("plugin-command-mcp-structured-workspace");
  let plugin_manifest = source_root.join("pith-plugin.json");
  let server_path = source_root.join("mcp-server.sh");
  fs::write(
    &plugin_manifest,
    r#"{
  "name": "mcp-structured",
  "version": "0.1.0",
  "displayName": "MCP Structured",
  "description": "MCP command plugin with Pith structured output",
  "author": { "name": "Pith" },
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
  "description": "Return Pith timeline and memory output through MCP structured content.",
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
{"jsonrpc":"2.0","id":2,"result":{"structuredContent":{"items":[{"kind":"pluginResult","title":"MCP Structured Item","content":"Owned MCP timeline item.","attributes":{"runner":"mcp"}}],"memoryNotes":[{"title":"MCP Structured Memory","body":"MCP structured content can store Pith memory.","source":"plugin.mcp-structured","tags":["mcp","structured"]}]}}}
JSON
"#,
  )
  .expect("write mcp structured server");
  let mut permissions = fs::metadata(&server_path)
    .expect("mcp structured server metadata")
    .permissions();
  permissions.set_mode(0o755);
  fs::set_permissions(&server_path, permissions).expect("set mcp structured server permissions");
  replace_plugin_catalog(
    &mut context,
    vec![PluginCatalogEntry {
      id: "mcp-structured".to_string(),
      name: "mcp-structured".to_string(),
      version: "0.1.0".to_string(),
      display_name: "MCP Structured".to_string(),
      status: "ready".to_string(),
      description: "MCP command plugin with Pith structured output".to_string(),
      author_name: Some("Pith".to_string()),
      enabled: true,
      default_enabled: true,
      capabilities: vec![
        "command:mcp-structured.capture".to_string(),
        "mcp_server:local".to_string(),
      ],
      permissions: vec!["mcp.connect".to_string()],
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

  fs::remove_dir_all(&workspace).expect("cleanup temp workspace");
  fs::remove_dir_all(source_root.parent().expect("plugin root")).expect("cleanup plugin source");

  assert!(response.error.is_none());
  let result = response.result.expect("command run result");
  let items = result["items"].as_array().expect("items");
  assert_eq!(items[0]["kind"], "pluginCommand");
  assert_eq!(items[1]["title"], "MCP Structured Item");
  assert_eq!(items[1]["attributes"]["runner"], "mcp");
  assert_eq!(items[1]["attributes"]["mcpProtocolStatus"], "completed");
  assert_eq!(
    items[1]["attributes"]["mcpStructuredContentStatus"],
    "pithOutputEnvelope"
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
    .contains("MCP structured content can store Pith memory."));
}

#[cfg(unix)]
#[test]
fn plugin_command_run_accepts_mcp_text_pith_output() {
  use std::os::unix::fs::PermissionsExt;

  let mut context = RuntimeContext::new_in_memory();
  let source_root = create_temp_plugin_bundle(
    "plugin-command-mcp-text-pith-output",
    "mcp-text",
    "MCP Text",
  );
  let workspace = create_temp_workspace("plugin-command-mcp-text-workspace");
  let plugin_manifest = source_root.join("pith-plugin.json");
  let server_path = source_root.join("mcp-server.sh");
  fs::write(
    &plugin_manifest,
    r#"{
  "name": "mcp-text",
  "version": "0.1.0",
  "displayName": "MCP Text",
  "description": "MCP command plugin with text output",
  "author": { "name": "Pith" },
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
  "description": "Return a Pith envelope from MCP text content.",
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
  let mut permissions = fs::metadata(&server_path)
    .expect("mcp text server metadata")
    .permissions();
  permissions.set_mode(0o755);
  fs::set_permissions(&server_path, permissions).expect("set mcp text server permissions");
  replace_plugin_catalog(
    &mut context,
    vec![PluginCatalogEntry {
      id: "mcp-text".to_string(),
      name: "mcp-text".to_string(),
      version: "0.1.0".to_string(),
      display_name: "MCP Text".to_string(),
      status: "ready".to_string(),
      description: "MCP command plugin with text output".to_string(),
      author_name: Some("Pith".to_string()),
      enabled: true,
      default_enabled: true,
      capabilities: vec![
        "command:mcp-text.capture".to_string(),
        "mcp_server:local".to_string(),
      ],
      permissions: vec!["mcp.connect".to_string()],
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

  fs::remove_dir_all(&workspace).expect("cleanup temp workspace");
  fs::remove_dir_all(source_root.parent().expect("plugin root")).expect("cleanup plugin source");

  assert!(response.error.is_none());
  let result = response.result.expect("command run result");
  let items = result["items"].as_array().expect("items");
  assert_eq!(items[0]["kind"], "pluginCommand");
  assert_eq!(items[1]["kind"], "pluginResult");
  assert_eq!(items[1]["content"], "MCP text envelope captured.");
  assert_eq!(items[1]["attributes"]["mcpProtocolStatus"], "completed");
  assert_eq!(
    items[1]["attributes"]["mcpContentStatus"],
    "pithOutputEnvelope"
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
  use std::os::unix::fs::PermissionsExt;

  let mut context = RuntimeContext::new_in_memory();
  let source_root = create_temp_plugin_bundle(
    "plugin-command-mcp-generic-structured-output",
    "mcp-generic",
    "MCP Generic",
  );
  let workspace = create_temp_workspace("plugin-command-mcp-generic-workspace");
  let plugin_manifest = source_root.join("pith-plugin.json");
  let server_path = source_root.join("mcp-server.sh");
  fs::write(
    &plugin_manifest,
    r#"{
  "name": "mcp-generic",
  "version": "0.1.0",
  "displayName": "MCP Generic",
  "description": "MCP command plugin with generic structured output",
  "author": { "name": "Pith" },
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
  let mut permissions = fs::metadata(&server_path)
    .expect("mcp generic server metadata")
    .permissions();
  permissions.set_mode(0o755);
  fs::set_permissions(&server_path, permissions).expect("set mcp generic server permissions");
  replace_plugin_catalog(
    &mut context,
    vec![PluginCatalogEntry {
      id: "mcp-generic".to_string(),
      name: "mcp-generic".to_string(),
      version: "0.1.0".to_string(),
      display_name: "MCP Generic".to_string(),
      status: "ready".to_string(),
      description: "MCP command plugin with generic structured output".to_string(),
      author_name: Some("Pith".to_string()),
      enabled: true,
      default_enabled: true,
      capabilities: vec![
        "command:mcp-generic.inspect".to_string(),
        "mcp_server:local".to_string(),
      ],
      permissions: vec!["mcp.connect".to_string()],
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

  fs::remove_dir_all(&workspace).expect("cleanup temp workspace");
  fs::remove_dir_all(source_root.parent().expect("plugin root")).expect("cleanup plugin source");

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
fn plugin_command_run_classifies_malformed_mcp_pith_output() {
  use std::os::unix::fs::PermissionsExt;

  let mut context = RuntimeContext::new_in_memory();
  let source_root = create_temp_plugin_bundle(
    "plugin-command-mcp-malformed-pith-output",
    "mcp-malformed",
    "MCP Malformed",
  );
  let workspace = create_temp_workspace("plugin-command-mcp-malformed-workspace");
  let plugin_manifest = source_root.join("pith-plugin.json");
  let server_path = source_root.join("mcp-server.sh");
  fs::write(
    &plugin_manifest,
    r#"{
  "name": "mcp-malformed",
  "version": "0.1.0",
  "displayName": "MCP Malformed",
  "description": "MCP command plugin with malformed Pith structured output",
  "author": { "name": "Pith" },
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
  "description": "Return malformed Pith MCP structured content.",
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
  let mut permissions = fs::metadata(&server_path)
    .expect("mcp malformed server metadata")
    .permissions();
  permissions.set_mode(0o755);
  fs::set_permissions(&server_path, permissions).expect("set mcp malformed server permissions");
  replace_plugin_catalog(
    &mut context,
    vec![PluginCatalogEntry {
      id: "mcp-malformed".to_string(),
      name: "mcp-malformed".to_string(),
      version: "0.1.0".to_string(),
      display_name: "MCP Malformed".to_string(),
      status: "ready".to_string(),
      description: "MCP command plugin with malformed Pith structured output".to_string(),
      author_name: Some("Pith".to_string()),
      enabled: true,
      default_enabled: true,
      capabilities: vec![
        "command:mcp-malformed.inspect".to_string(),
        "mcp_server:local".to_string(),
      ],
      permissions: vec!["mcp.connect".to_string()],
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

  fs::remove_dir_all(&workspace).expect("cleanup temp workspace");
  fs::remove_dir_all(source_root.parent().expect("plugin root")).expect("cleanup plugin source");

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
    "pithOutputEnvelope"
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
  use std::os::unix::fs::PermissionsExt;

  let mut context = RuntimeContext::new_in_memory();
  let source_root = create_temp_plugin_bundle(
    "plugin-command-mcp-diagnostics",
    "mcp-diagnostics",
    "MCP Diagnostics",
  );
  let workspace = create_temp_workspace("plugin-command-mcp-diagnostics-workspace");
  let plugin_manifest = source_root.join("pith-plugin.json");
  let server_path = source_root.join("mcp-server.sh");
  fs::write(
    &plugin_manifest,
    r#"{
  "name": "mcp-diagnostics",
  "version": "0.1.0",
  "displayName": "MCP Diagnostics",
  "description": "MCP protocol diagnostics plugin",
  "author": { "name": "Pith" },
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
  let mut permissions = fs::metadata(&server_path)
    .expect("mcp diagnostics server metadata")
    .permissions();
  permissions.set_mode(0o755);
  fs::set_permissions(&server_path, permissions).expect("set mcp diagnostics server permissions");
  replace_plugin_catalog(
    &mut context,
    vec![PluginCatalogEntry {
      id: "mcp-diagnostics".to_string(),
      name: "mcp-diagnostics".to_string(),
      version: "0.1.0".to_string(),
      display_name: "MCP Diagnostics".to_string(),
      status: "ready".to_string(),
      description: "MCP protocol diagnostics plugin".to_string(),
      author_name: Some("Pith".to_string()),
      enabled: true,
      default_enabled: true,
      capabilities: vec![
        "command:mcp-diagnostics.run".to_string(),
        "mcp_server:local".to_string(),
      ],
      permissions: vec!["mcp.connect".to_string()],
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

  fs::remove_dir_all(&workspace).expect("cleanup temp workspace");
  fs::remove_dir_all(source_root.parent().expect("plugin root")).expect("cleanup plugin source");

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
    "Make the MCP server return a JSON-RPC tools/call response with the expected request id."
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
    "pith.plugin.command.input"
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
  let plugin_manifest = source_root.join("pith-plugin.json");
  let server_path = source_root.join("mcp-server.sh");
  fs::write(
    &plugin_manifest,
    r#"{
  "name": "mcp-permission",
  "version": "0.1.0",
  "displayName": "MCP Permission",
  "description": "MCP command plugin missing permissions",
  "author": { "name": "Pith" },
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
    use std::os::unix::fs::PermissionsExt;

    let mut permissions = fs::metadata(&server_path)
      .expect("mcp server metadata")
      .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&server_path, permissions).expect("make mcp server executable");
  }
  replace_plugin_catalog(
    &mut context,
    vec![PluginCatalogEntry {
      id: "mcp-permission".to_string(),
      name: "mcp-permission".to_string(),
      version: "0.1.0".to_string(),
      display_name: "MCP Permission".to_string(),
      status: "ready".to_string(),
      description: "MCP command plugin missing permissions".to_string(),
      author_name: Some("Pith".to_string()),
      enabled: true,
      default_enabled: true,
      capabilities: vec![
        "command:mcp-permission.run".to_string(),
        "mcp_server:local".to_string(),
      ],
      permissions: vec![],
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

  fs::remove_dir_all(&workspace).expect("cleanup temp workspace");
  fs::remove_dir_all(source_root.parent().expect("plugin root")).expect("cleanup plugin source");

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
  let plugin_manifest = source_root.join("pith-plugin.json");
  let server_path = source_root.join("mcp-server.sh");
  fs::write(
    &plugin_manifest,
    r#"{
  "name": "mcp-network",
  "version": "0.1.0",
  "displayName": "MCP Network",
  "description": "Connector MCP command plugin missing network permission",
  "author": { "name": "Pith" },
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
    "credentialStore": "keychain"
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
    use std::os::unix::fs::PermissionsExt;

    let mut permissions = fs::metadata(&server_path)
      .expect("mcp server metadata")
      .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&server_path, permissions).expect("make mcp server executable");
  }
  replace_plugin_catalog(
    &mut context,
    vec![PluginCatalogEntry {
      id: "mcp-network".to_string(),
      name: "mcp-network".to_string(),
      version: "0.1.0".to_string(),
      display_name: "MCP Network".to_string(),
      status: "ready".to_string(),
      description: "Connector MCP command plugin missing network permission".to_string(),
      author_name: Some("Pith".to_string()),
      enabled: true,
      default_enabled: true,
      capabilities: vec![
        "command:mcp-network.sync".to_string(),
        "mcp_server:notion".to_string(),
        "connector:notion".to_string(),
      ],
      permissions: vec!["mcp.connect".to_string()],
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

  fs::remove_dir_all(&workspace).expect("cleanup temp workspace");
  fs::remove_dir_all(source_root.parent().expect("plugin root")).expect("cleanup plugin source");

  let error = response.error.expect("permission blocker error");
  assert_eq!(error.code, -32053);
  assert!(error.message.contains("network.outbound"));
}

#[cfg(unix)]
#[test]
fn plugin_command_run_records_stdio_runner_failure() {
  use std::os::unix::fs::PermissionsExt;

  let mut context = RuntimeContext::new_in_memory();
  let source_root = create_temp_plugin_bundle(
    "plugin-command-stdio-failure",
    "stdio-failure",
    "Stdio Failure",
  );
  let workspace = create_temp_workspace("plugin-command-stdio-failure-workspace");
  let plugin_manifest = source_root.join("pith-plugin.json");
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
  let mut permissions = fs::metadata(&runner_path)
    .expect("runner metadata")
    .permissions();
  permissions.set_mode(0o755);
  fs::set_permissions(&runner_path, permissions).expect("set runner permissions");
  replace_plugin_catalog(
    &mut context,
    vec![PluginCatalogEntry {
      id: "stdio-failure".to_string(),
      name: "stdio-failure".to_string(),
      version: "0.1.0".to_string(),
      display_name: "Stdio Failure".to_string(),
      status: "ready".to_string(),
      description: "Failing stdio command plugin".to_string(),
      author_name: Some("Pith".to_string()),
      enabled: true,
      default_enabled: true,
      capabilities: vec!["command:stdio-failure.run".to_string()],
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

  fs::remove_dir_all(&workspace).expect("cleanup temp workspace");
  fs::remove_dir_all(source_root.parent().expect("plugin root")).expect("cleanup plugin source");

  assert!(response.error.is_none());
  let result = response.result.expect("command run result");
  let items = result["items"].as_array().expect("items");
  assert_eq!(items[1]["kind"], "warning");
  assert_eq!(items[1]["attributes"]["pluginCommandStatus"], "failed");
  assert_eq!(items[1]["attributes"]["pluginRunnerErrorCode"], "-32054");
  assert_eq!(
    items[1]["attributes"]["pluginRunnerFailureKind"],
    "processExit"
  );
  assert_eq!(
    items[1]["attributes"]["pluginRunnerRecoveryHint"],
    "Fix the runner error shown in stderr/stdout, then return exit code 0 with valid output."
  );
  assert_eq!(
    items[1]["attributes"]["pluginRunnerExecutionDriver"],
    "stdio"
  );
  assert_eq!(
    items[1]["attributes"]["pluginRunnerExecutionKind"],
    "stdio.failure"
  );
  assert_eq!(
    items[1]["attributes"]["pluginRunnerEntrypoint"],
    "runner.sh"
  );
  assert_eq!(
    items[1]["attributes"]["pluginRunnerInputEnvelope"],
    "pith.plugin.command.input"
  );
  assert_eq!(
    items[1]["attributes"]["pluginRunnerInputFieldNames"],
    "threadId, input, workspace"
  );
  assert_eq!(
    items[1]["attributes"]["pluginRunnerInputRequiredFields"],
    "threadId"
  );
  assert_eq!(items[1]["attributes"]["pluginRunnerInputProvided"], "true");
  assert_eq!(items[1]["attributes"]["pluginRunnerInputBytes"], "11");
  assert_eq!(
    items[1]["attributes"]["pluginRunnerOutputEnvelope"],
    "pith.plugin.command.output"
  );
  assert_eq!(items[1]["attributes"]["pluginRunnerOutputFieldCount"], "4");
  assert_eq!(
    items[1]["attributes"]["pluginRunnerOutputFieldNames"],
    "content, message, items, memoryNotes"
  );
  assert!(items[1]["attributes"]["pluginRunnerOutputRequiredFields"].is_null());
  assert_eq!(
    items[1]["attributes"]["pluginRunnerEntrypointCheck"],
    "ready"
  );
  assert_eq!(
    items[1]["attributes"]["pluginRunnerEntrypointFileKind"],
    "file"
  );
  assert_eq!(
    items[1]["attributes"]["pluginRunnerEntrypointExecutable"],
    "true"
  );
  assert!(items[1]["attributes"]["pluginRunnerPluginRoot"].is_string());
  assert!(items[1]["attributes"]["pluginRunnerResolvedEntrypoint"].is_string());
  assert_eq!(items[1]["attributes"]["pluginRunnerExitCode"], "7");
  assert_eq!(
    items[1]["attributes"]["pluginRunnerExitReason"],
    "completed"
  );
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
  use std::os::unix::fs::PermissionsExt;

  let mut context = RuntimeContext::new_in_memory();
  let source_root = create_temp_plugin_bundle(
    "plugin-command-empty-output",
    "empty-output",
    "Empty Output",
  );
  let workspace = create_temp_workspace("plugin-command-empty-output-workspace");
  let plugin_manifest = source_root.join("pith-plugin.json");
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
  let mut permissions = fs::metadata(&runner_path)
    .expect("runner metadata")
    .permissions();
  permissions.set_mode(0o755);
  fs::set_permissions(&runner_path, permissions).expect("set runner permissions");
  replace_plugin_catalog(
    &mut context,
    vec![PluginCatalogEntry {
      id: "empty-output".to_string(),
      name: "empty-output".to_string(),
      version: "0.1.0".to_string(),
      display_name: "Empty Output".to_string(),
      status: "ready".to_string(),
      description: "Empty output command plugin".to_string(),
      author_name: Some("Pith".to_string()),
      enabled: true,
      default_enabled: true,
      capabilities: vec!["command:empty-output.run".to_string()],
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

  fs::remove_dir_all(&workspace).expect("cleanup temp workspace");
  fs::remove_dir_all(source_root.parent().expect("plugin root")).expect("cleanup plugin source");

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
    items[1]["attributes"]["pluginRunnerRecoveryHint"],
    "Populate `content`, `message`, `items`, or `memoryNotes`, or return plain text."
  );
  assert_eq!(
    items[1]["attributes"]["pluginRunnerOutputStatus"],
    "emptyEnvelope"
  );
  assert_eq!(items[1]["attributes"]["pluginRunnerOutputParsed"], "true");
  assert_eq!(
    items[1]["attributes"]["pluginRunnerOutputContentBytes"],
    "0"
  );
  assert_eq!(
    items[1]["attributes"]["pluginRunnerOutputValidTimelineItemCount"],
    "0"
  );
  assert_eq!(
    items[1]["attributes"]["pluginRunnerOutputInvalidTimelineItemCount"],
    "0"
  );
  assert!(items[1]["content"]
    .as_str()
    .unwrap()
    .contains("without content, valid timeline items, or memory notes"));
}

#[cfg(unix)]
#[test]
fn plugin_command_run_rejects_malformed_stdio_output_envelope() {
  use std::os::unix::fs::PermissionsExt;

  let mut context = RuntimeContext::new_in_memory();
  let source_root = create_temp_plugin_bundle(
    "plugin-command-malformed-output",
    "malformed-output",
    "Malformed Output",
  );
  let workspace = create_temp_workspace("plugin-command-malformed-output-workspace");
  let plugin_manifest = source_root.join("pith-plugin.json");
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
  let mut permissions = fs::metadata(&runner_path)
    .expect("runner metadata")
    .permissions();
  permissions.set_mode(0o755);
  fs::set_permissions(&runner_path, permissions).expect("set runner permissions");
  replace_plugin_catalog(
    &mut context,
    vec![PluginCatalogEntry {
      id: "malformed-output".to_string(),
      name: "malformed-output".to_string(),
      version: "0.1.0".to_string(),
      display_name: "Malformed Output".to_string(),
      status: "ready".to_string(),
      description: "Malformed output command plugin".to_string(),
      author_name: Some("Pith".to_string()),
      enabled: true,
      default_enabled: true,
      capabilities: vec!["command:malformed-output.run".to_string()],
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

  fs::remove_dir_all(&workspace).expect("cleanup temp workspace");
  fs::remove_dir_all(source_root.parent().expect("plugin root")).expect("cleanup plugin source");

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
  use std::os::unix::fs::PermissionsExt;

  let mut context = RuntimeContext::new_in_memory();
  let source_root = create_temp_plugin_bundle(
    "plugin-command-runner-memory",
    "runner-memory",
    "Runner Memory",
  );
  let workspace = create_temp_workspace("plugin-command-runner-memory-workspace");
  let plugin_manifest = source_root.join("pith-plugin.json");
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
  let mut permissions = fs::metadata(&runner_path)
    .expect("runner metadata")
    .permissions();
  permissions.set_mode(0o755);
  fs::set_permissions(&runner_path, permissions).expect("set runner permissions");
  replace_plugin_catalog(
    &mut context,
    vec![PluginCatalogEntry {
      id: "runner-memory".to_string(),
      name: "runner-memory".to_string(),
      version: "0.1.0".to_string(),
      display_name: "Runner Memory".to_string(),
      status: "ready".to_string(),
      description: "Runner memory command plugin".to_string(),
      author_name: Some("Pith".to_string()),
      enabled: true,
      default_enabled: true,
      capabilities: vec!["command:runner-memory.run".to_string()],
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

  fs::remove_dir_all(&workspace).expect("cleanup temp workspace");
  fs::remove_dir_all(source_root.parent().expect("plugin root")).expect("cleanup plugin source");

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
  assert_eq!(
    items[1]["attributes"]["pluginRunnerOutputMemoryNoteCount"],
    "1"
  );
  assert_eq!(
    items[1]["attributes"]["pluginRunnerOutputInvalidMemoryNoteCount"],
    "0"
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
  let plugin_manifest = source_root.join("pith-plugin.json");
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
    vec![PluginCatalogEntry {
      id: "non-exec".to_string(),
      name: "non-exec".to_string(),
      version: "0.1.0".to_string(),
      display_name: "Non Exec".to_string(),
      status: "ready".to_string(),
      description: "Non executable stdio command plugin".to_string(),
      author_name: Some("Pith".to_string()),
      enabled: true,
      default_enabled: true,
      capabilities: vec!["command:non-exec.run".to_string()],
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

  fs::remove_dir_all(&workspace).expect("cleanup temp workspace");
  fs::remove_dir_all(source_root.parent().expect("plugin root")).expect("cleanup plugin source");

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
  assert_eq!(items[1]["attributes"]["sandboxMode"], "workspaceReadWrite");
  assert!(items[1]["attributes"]["sandboxBackend"].is_string());
  assert_eq!(items[1]["attributes"]["executionKind"], "stdio.ownedItems");
  assert_eq!(
    items[1]["attributes"]["pluginRunnerOutputStatus"],
    "envelope"
  );
  assert_eq!(
    items[1]["attributes"]["pluginRunnerOutputValidTimelineItemCount"],
    "1"
  );
  assert_eq!(
    items[1]["attributes"]["pluginRunnerOutputInvalidTimelineItemCount"],
    "0"
  );
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
