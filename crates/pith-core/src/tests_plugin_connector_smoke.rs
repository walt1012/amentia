#[cfg(unix)]
use super::test_support::{create_temp_plugin_bundle, create_temp_workspace, request};
#[cfg(unix)]
use super::{handle_request, RuntimeContext};
#[cfg(unix)]
use pith_protocol::methods;
#[cfg(unix)]
use serde_json::{json, Value};
#[cfg(unix)]
use std::fs;
#[cfg(unix)]
use std::path::{Path, PathBuf};

#[cfg(unix)]
#[test]
fn third_party_connector_plugin_smoke_path_supports_repair_and_retry() {
  let mut context = RuntimeContext::new_in_memory();
  let source_root = create_repairable_connector_plugin("third-party-connector-smoke");
  let install_root = create_temp_workspace("third-party-connector-install-root");
  let workspace = create_temp_workspace("third-party-connector-workspace");
  context
    .plugin_state
    .configure_roots(vec![install_root.clone()], install_root.clone());

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
        "title": "Third Party Connector Smoke"
      })),
    ),
  );

  let inspect_response = handle_request(
    &mut context,
    request(
      methods::PLUGIN_INSPECT,
      Some(json!({
        "sourcePath": source_root.display().to_string()
      })),
    ),
  );
  assert!(inspect_response.error.is_none());
  let inspect_result = inspect_response.result.expect("inspect result");
  assert_eq!(inspect_result["plugin"]["id"], "notion-smoke");
  assert_eq!(inspect_result["installStatus"], "ready");
  assert_eq!(inspect_result["plugin"]["enabled"], false);
  assert!(context.plugin_state.catalog().is_empty());

  let install_response = handle_request(
    &mut context,
    request(
      methods::PLUGIN_INSTALL,
      Some(json!({
        "sourcePath": source_root.display().to_string()
      })),
    ),
  );
  assert!(install_response.error.is_none());
  let installed_plugin = install_response.result.expect("install result")["plugin"].clone();
  assert_eq!(installed_plugin["id"], "notion-smoke");
  assert_eq!(installed_plugin["enabled"], false);
  let installed_runner = installed_runner_path(&installed_plugin);

  let enable_response = handle_request(
    &mut context,
    request(
      methods::PLUGIN_SET_ENABLED,
      Some(json!({
        "pluginId": "notion-smoke",
        "enabled": true
      })),
    ),
  );
  assert!(enable_response.error.is_none());
  assert_eq!(
    enable_response.result.expect("enable result")["plugin"]["enabled"],
    true
  );

  let connector_registry = handle_request(
    &mut context,
    request(methods::PLUGIN_CONNECTOR_REGISTRY, None),
  );
  assert!(connector_registry.error.is_none());
  let connector_registry_result = connector_registry
    .result
    .expect("connector registry result");
  let connector = &connector_registry_result["connectors"][0];
  assert_eq!(connector["connectorId"], "notion-smoke::notion");
  assert_eq!(connector["status"], "needsAuth");
  assert_eq!(connector["authStatus"], "needsAuth");
  assert_eq!(connector["credentialPresent"], false);

  let authorize_response = handle_request(
    &mut context,
    request(
      methods::PLUGIN_CONNECTOR_AUTHORIZE,
      Some(json!({
        "connectorId": "notion-smoke::notion",
        "credentialSecret": "smoke-token"
      })),
    ),
  );
  assert!(authorize_response.error.is_none());
  let authorized_connector =
    authorize_response.result.expect("authorize result")["connector"].clone();
  assert_eq!(authorized_connector["status"], "ready");
  assert_eq!(authorized_connector["authStatus"], "authorized");
  assert_eq!(authorized_connector["credentialSecretPresent"], true);

  let first_run = run_smoke_command(&mut context);
  let first_approval_id = pending_approval_id(&first_run);
  let failed_approval = approve(&mut context, &first_approval_id);
  let failed_items = failed_approval["items"].as_array().expect("failed items");
  assert_eq!(failed_items[0]["kind"], "approvalResolved");
  assert_eq!(failed_items[0]["attributes"]["decision"], "approved");
  assert_eq!(
    failed_items[0]["attributes"]["commandId"],
    "notion-smoke::notion-smoke.sync"
  );
  let failed_item = failed_items
    .iter()
    .find(|item| item["attributes"]["pluginCommandStatus"] == "failed")
    .expect("failed plugin item");
  assert_eq!(failed_item["kind"], "warning");
  assert_eq!(
    failed_item["attributes"]["pluginRunnerFailureKind"],
    "processExit"
  );
  assert_eq!(
    failed_item["attributes"]["connectorId"],
    "notion-smoke::notion"
  );
  assert_eq!(failed_item["attributes"]["commandInput"], "sync retry");
  assert_eq!(
    failed_item["attributes"]["connectorSecretBindings"],
    "env-bound"
  );
  assert!(failed_item["attributes"]["sourcePath"]
    .as_str()
    .expect("source path")
    .contains("notion-smoke.sync.json"));
  assert!(failed_item["attributes"]["pluginRunnerRecoveryHint"]
    .as_str()
    .expect("recovery hint")
    .contains("Fix the runner error"));
  assert!(failed_item["content"]
    .as_str()
    .expect("failure content")
    .contains("temporary connector failure"));

  write_runner(
    &installed_runner,
    r#"#!/bin/sh
payload=$(cat)
case "$payload" in *'"connectorId":"notion-smoke::notion"'*) connector=true;; *) connector=false;; esac
case "$payload" in *'"input":"sync retry"'*) input=true;; *) input=false;; esac
if [ "$PITH_PLUGIN_CREDENTIAL_1_NOTION_SMOKE__NOTION" = "smoke-token" ]; then credential_env=true; else credential_env=false; fi
case "$payload" in *"smoke-token"*) token_leak=true;; *) token_leak=false;; esac
printf '{"content":"connector=%s input=%s credentialEnv=%s tokenLeak=%s"}\n' "$connector" "$input" "$credential_env" "$token_leak"
"#,
  );

  let retry_run = run_smoke_command(&mut context);
  let retry_approval_id = pending_approval_id(&retry_run);
  let retry_approval = approve(&mut context, &retry_approval_id);
  let retry_items = retry_approval["items"].as_array().expect("retry items");
  let result_item = retry_items
    .iter()
    .find(|item| item["kind"] == "pluginResult")
    .expect("plugin result item");
  assert_eq!(result_item["attributes"]["pluginRunnerConnectorCount"], "1");
  assert_eq!(
    result_item["attributes"]["pluginRunnerSecretBindings"],
    "env-bound"
  );
  assert_eq!(
    result_item["content"],
    "connector=true input=true credentialEnv=true tokenLeak=false"
  );

  fs::remove_dir_all(&workspace).expect("cleanup workspace");
  fs::remove_dir_all(&install_root).expect("cleanup install root");
  fs::remove_dir_all(source_root.parent().expect("plugin source root"))
    .expect("cleanup source root");
}

#[cfg(unix)]
fn create_repairable_connector_plugin(label: &str) -> PathBuf {
  let source_root = create_temp_plugin_bundle(label, "notion-smoke", "Notion Smoke");
  fs::write(
    source_root.join("pith-plugin.json"),
    r#"{
  "name": "notion-smoke",
  "version": "0.1.0",
  "displayName": "Notion Smoke",
  "description": "Third-party connector smoke plugin",
  "author": { "name": "Pith" },
  "capabilities": ["command:notion-smoke.sync", "connector:notion"],
  "permissions": ["network.outbound"],
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
  "defaultEnabled": false
}"#,
  )
  .expect("write plugin manifest");
  fs::write(
    source_root.join("commands").join("notion-smoke.sync.json"),
    r#"{
  "title": "Sync Notion Smoke",
  "description": "Exercise the third-party connector smoke path.",
  "prompt": "Sync the local smoke test with Notion.",
  "execution": {
    "kind": "stdio.notionSmoke",
    "entrypoint": "runner.sh",
    "connectors": ["notion"]
  }
}"#,
  )
  .expect("write command manifest");
  write_runner(
    &source_root.join("runner.sh"),
    r#"#!/bin/sh
cat >/dev/null
echo "temporary connector failure" >&2
exit 7
"#,
  );
  source_root
}

#[cfg(unix)]
fn run_smoke_command(context: &mut RuntimeContext) -> Value {
  let response = handle_request(
    context,
    request(
      methods::PLUGIN_COMMAND_RUN,
      Some(json!({
        "threadId": "thread-1",
        "commandId": "notion-smoke::notion-smoke.sync",
        "input": "sync retry"
      })),
    ),
  );
  assert!(response.error.is_none());
  let result = response.result.expect("command run result");
  let items = result["items"].as_array().expect("command items");
  assert_eq!(items[0]["kind"], "pluginCommand");
  assert_eq!(items[1]["kind"], "approvalRequested");
  assert_eq!(
    items[1]["attributes"]["connectorId"],
    "notion-smoke::notion"
  );
  assert_eq!(items[1]["attributes"]["commandInput"], "sync retry");
  result
}

#[cfg(unix)]
fn approve(context: &mut RuntimeContext, approval_id: &str) -> Value {
  let response = handle_request(
    context,
    request(
      methods::APPROVAL_RESPOND,
      Some(json!({
        "approvalId": approval_id,
        "decision": "approved"
      })),
    ),
  );
  assert!(response.error.is_none());
  response.result.expect("approval result")
}

#[cfg(unix)]
fn pending_approval_id(result: &Value) -> String {
  let approvals = result["pendingApprovals"]
    .as_array()
    .expect("pending approvals");
  assert_eq!(approvals.len(), 1);
  approvals[0]["id"]
    .as_str()
    .expect("approval id")
    .to_string()
}

#[cfg(unix)]
fn installed_runner_path(installed_plugin: &Value) -> PathBuf {
  let manifest_path = installed_plugin["manifestPath"]
    .as_str()
    .expect("manifest path");
  PathBuf::from(manifest_path)
    .parent()
    .expect("installed plugin root")
    .join("runner.sh")
}

#[cfg(unix)]
fn write_runner(path: &Path, content: &str) {
  use std::os::unix::fs::PermissionsExt;

  fs::write(path, content).expect("write runner");
  let mut permissions = fs::metadata(path).expect("runner metadata").permissions();
  permissions.set_mode(0o755);
  fs::set_permissions(path, permissions).expect("set runner permissions");
}
