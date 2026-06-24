use super::test_support::{
  bundled_plugin_entry, create_temp_plugin_bundle, create_temp_workspace, replace_plugin_catalog,
  request,
};
use super::*;
use amentia_protocol::methods;
use amentia_storage::RuntimeStore;
use serde_json::json;
use std::fs;

#[test]
fn plugin_set_enabled_updates_runtime_catalog() {
  let mut context = RuntimeContext::new_in_memory();
  replace_plugin_catalog(
    &mut context,
    vec![bundled_plugin_entry(
      "workspace-notes",
      "Workspace Notes",
      false,
      false,
      &["skill:workspace.notes"],
      &["file.read"],
    )],
  );

  let response = handle_request(
    &mut context,
    request(
      methods::PLUGIN_SET_ENABLED,
      Some(json!({
        "pluginId": "workspace-notes",
        "enabled": true
      })),
    ),
  );

  assert!(response.error.is_none());
  assert!(context.plugin_state.catalog()[0].enabled);
  assert_eq!(
    response.result.expect("plugin set result")["plugin"]["enabled"],
    true
  );
}

#[test]
fn plugin_set_enabled_does_not_mutate_catalog_when_persistence_fails() {
  let mut context = RuntimeContext::new_in_memory();
  let storage_root = create_temp_workspace("plugin-enable-failing-storage");
  let database_path = storage_root.join("amentia.db");
  fs::create_dir_all(&database_path).expect("create directory at database path");
  context
    .persistence_state
    .set_store_for_testing(RuntimeStore::new(database_path));
  replace_plugin_catalog(
    &mut context,
    vec![bundled_plugin_entry(
      "workspace-notes",
      "Workspace Notes",
      false,
      false,
      &["skill:workspace.notes"],
      &["file.read"],
    )],
  );

  let response = handle_request(
    &mut context,
    request(
      methods::PLUGIN_SET_ENABLED,
      Some(json!({
        "pluginId": "workspace-notes",
        "enabled": true
      })),
    ),
  );

  fs::remove_dir_all(&storage_root).expect("cleanup storage root");

  assert!(response.result.is_none());
  let error = response.error.expect("plugin enable error");
  assert_eq!(error.code, -32010);
  let data = error.data.expect("plugin enable error data");
  assert_eq!(data["pluginId"], "workspace-notes");
  assert_eq!(data["pluginLifecycleOperation"], "enable");
  assert_eq!(data["pluginLifecycleStatus"], "persistFailed");
  assert!(
    data["lifecycleBlocker"]
      .as_str()
      .expect("lifecycle blocker")
      .len()
      > 10
  );
  assert!(data["lifecycleRepairHint"]
    .as_str()
    .expect("lifecycle repair hint")
    .contains("storage permissions"));
  assert!(!context.plugin_state.catalog()[0].enabled);
}

#[test]
fn plugin_inspect_previews_local_plugin_without_installing() {
  let mut context = RuntimeContext::new_in_memory();
  let source_root =
    create_temp_plugin_bundle("plugin-inspect-source", "focus-review", "Focus Review");
  replace_plugin_catalog(&mut context, vec![]);

  let response = handle_request(
    &mut context,
    request(
      methods::PLUGIN_INSPECT,
      Some(json!({
        "sourcePath": source_root.display().to_string()
      })),
    ),
  );

  fs::remove_dir_all(source_root.parent().expect("plugin source root"))
    .expect("cleanup plugin source root");

  assert!(response.error.is_none());
  let result = response.result.expect("plugin inspect result");
  assert_eq!(result["plugin"]["id"], "focus-review");
  assert_eq!(result["plugin"]["displayName"], "Focus Review");
  assert_eq!(result["plugin"]["provenance"], "local");
  assert_eq!(result["installStatus"], "ready");
  assert!(result.get("installBlocker").is_none());
  assert!(context.plugin_state.catalog().is_empty());
}

#[test]
fn plugin_inspect_reports_duplicate_install_blocker() {
  let mut context = RuntimeContext::new_in_memory();
  let source_root = create_temp_plugin_bundle(
    "plugin-inspect-duplicate",
    "workspace-notes",
    "Workspace Notes",
  );
  replace_plugin_catalog(
    &mut context,
    vec![bundled_plugin_entry(
      "workspace-notes",
      "Workspace Notes",
      true,
      true,
      &["skill:workspace.notes"],
      &["file.read"],
    )],
  );

  let response = handle_request(
    &mut context,
    request(
      methods::PLUGIN_INSPECT,
      Some(json!({
        "sourcePath": source_root.display().to_string()
      })),
    ),
  );

  fs::remove_dir_all(source_root.parent().expect("plugin source root"))
    .expect("cleanup plugin source root");

  assert!(response.error.is_none());
  let result = response.result.expect("plugin inspect result");
  assert_eq!(result["plugin"]["id"], "workspace-notes");
  assert_eq!(result["installStatus"], "alreadyInstalled");
  assert!(result["installBlocker"]
    .as_str()
    .expect("install blocker")
    .contains("already installed"));
  assert!(result["installRepairHint"]
    .as_str()
    .expect("install repair hint")
    .contains("Remove the existing local plugin first"));
}

#[test]
fn plugin_inspect_reports_structured_manifest_repair_data() {
  let mut context = RuntimeContext::new_in_memory();
  let source_root =
    create_temp_plugin_bundle("plugin-inspect-invalid", "focus-review", "Focus Review");
  fs::write(
    source_root.join("amentia-plugin.json"),
    r#"{
"name": "focus review",
"version": "0.1.0",
"displayName": "Focus Review",
"description": "Invalid test plugin",
"author": { "name": "Amentia" },
"capabilities": ["command:focus-review.run"],
"permissions": ["file.read"],
"defaultEnabled": true
}"#,
  )
  .expect("write invalid plugin manifest");
  let source_path = source_root.display().to_string();
  replace_plugin_catalog(&mut context, vec![]);

  let response = handle_request(
    &mut context,
    request(
      methods::PLUGIN_INSPECT,
      Some(json!({
        "sourcePath": source_path.clone()
      })),
    ),
  );

  fs::remove_dir_all(source_root.parent().expect("plugin source root"))
    .expect("cleanup plugin source root");

  assert!(response.result.is_none());
  let error = response.error.expect("plugin inspect error");
  assert_eq!(error.code, -32053);
  assert!(error.message.contains("must not contain whitespace"));
  assert!(!error.message.contains("Hint:"));
  let data = error.data.expect("plugin inspect error data");
  assert_eq!(data["sourcePath"], source_path.as_str());
  assert_eq!(data["pluginInstallStatus"], "inspectFailed");
  assert!(data["installBlocker"]
    .as_str()
    .expect("install blocker")
    .contains("must not contain whitespace"));
  assert!(data["installRepairHint"]
    .as_str()
    .expect("install repair hint")
    .contains("stable plugin identifiers"));
}

#[test]
fn plugin_install_adds_local_plugin_to_the_runtime_catalog() {
  let mut context = RuntimeContext::new_in_memory();
  let source_root =
    create_temp_plugin_bundle("plugin-install-source", "focus-review", "Focus Review");
  let install_root = create_temp_workspace("plugin-install-root");
  context
    .plugin_state
    .configure_roots(vec![install_root.clone()], install_root.clone());
  replace_plugin_catalog(&mut context, vec![]);

  let response = handle_request(
    &mut context,
    request(
      methods::PLUGIN_INSTALL,
      Some(json!({
        "sourcePath": source_root.display().to_string()
      })),
    ),
  );

  fs::remove_dir_all(source_root.parent().expect("plugin source root"))
    .expect("cleanup plugin source root");
  fs::remove_dir_all(&install_root).expect("cleanup install root");

  assert!(response.error.is_none());
  let result = response.result.expect("plugin install result");
  assert_eq!(result["plugin"]["id"], "focus-review");
  assert_eq!(result["plugin"]["provenance"], "local");
  assert!(context
    .plugin_state
    .catalog()
    .iter()
    .any(|plugin| plugin.id == "focus-review"));
}

#[test]
fn plugin_install_reports_refresh_failure_as_structured_recovery() {
  let mut context = RuntimeContext::new_in_memory();
  let storage_root = create_temp_workspace("plugin-install-refresh-failing-storage");
  let database_path = storage_root.join("amentia.db");
  fs::create_dir_all(&database_path).expect("create directory at database path");
  context
    .persistence_state
    .set_store_for_testing(RuntimeStore::new(database_path));
  let source_root = create_temp_plugin_bundle(
    "plugin-install-refresh-fail",
    "focus-review",
    "Focus Review",
  );
  let source_path = source_root.display().to_string();
  let install_root = create_temp_workspace("plugin-install-refresh-fail-root");
  context
    .plugin_state
    .configure_roots(vec![install_root.clone()], install_root.clone());
  replace_plugin_catalog(&mut context, vec![]);

  let response = handle_request(
    &mut context,
    request(
      methods::PLUGIN_INSTALL,
      Some(json!({
        "sourcePath": source_path.clone()
      })),
    ),
  );

  fs::remove_dir_all(source_root.parent().expect("plugin source root"))
    .expect("cleanup plugin source root");
  fs::remove_dir_all(&install_root).expect("cleanup install root");
  fs::remove_dir_all(&storage_root).expect("cleanup storage root");

  assert!(response.result.is_none());
  let error = response.error.expect("plugin install refresh error");
  assert_eq!(error.code, -32053);
  let data = error.data.expect("plugin install refresh error data");
  assert_eq!(data["sourcePath"], source_path.as_str());
  assert_eq!(data["pluginInstallStatus"], "refreshFailed");
  let install_blocker = data["installBlocker"].as_str().expect("install blocker");
  assert!(install_blocker.len() > 10);
}

#[test]
fn plugin_refresh_reloads_fixed_local_manifest() {
  let mut context = RuntimeContext::new_in_memory();
  let plugin_root = create_temp_plugin_bundle("plugin-refresh-fix", "refresh-demo", "Refresh Demo");
  let scan_root = plugin_root
    .parent()
    .expect("plugin scan root")
    .to_path_buf();
  context
    .plugin_state
    .configure_roots(vec![scan_root.clone()], scan_root.clone());
  fs::write(
    plugin_root.join("amentia-plugin.json"),
    r#"{
"name": "refresh-demo",
"version": "0.1.0",
"displayName": "Refresh Demo",
"description": "Refreshable local plugin",
"author": { "name": "Amentia" },
"capabilities": ["memory:refresh-demo"],
"permissions": ["file.read"],
"defaultEnabled": true
}"#,
  )
  .expect("write invalid plugin manifest");

  let invalid_response = handle_request(&mut context, request(methods::PLUGIN_REFRESH, None));
  assert!(invalid_response.error.is_none());
  let invalid_plugins = invalid_response.result.expect("invalid refresh result")["plugins"]
    .as_array()
    .expect("invalid plugin list")
    .clone();
  assert_eq!(invalid_plugins.len(), 1);
  assert_eq!(invalid_plugins[0]["status"], "invalid");
  assert!(invalid_plugins[0]["validationHint"]
    .as_str()
    .expect("validation hint")
    .contains("supported capability kinds"));

  fs::write(
    plugin_root.join("amentia-plugin.json"),
    r#"{
"name": "refresh-demo",
"version": "0.1.0",
"displayName": "Refresh Demo",
"description": "Refreshable local plugin",
"author": { "name": "Amentia" },
"capabilities": ["command:refresh-demo.run"],
"permissions": ["file.read"],
"defaultEnabled": true
}"#,
  )
  .expect("write fixed plugin manifest");

  let fixed_response = handle_request(&mut context, request(methods::PLUGIN_REFRESH, None));

  fs::remove_dir_all(&scan_root).expect("cleanup plugin root");

  assert!(fixed_response.error.is_none());
  let fixed_plugins = fixed_response.result.expect("fixed refresh result")["plugins"]
    .as_array()
    .expect("fixed plugin list")
    .clone();
  assert_eq!(fixed_plugins.len(), 1);
  assert_eq!(fixed_plugins[0]["id"], "refresh-demo");
  assert_eq!(fixed_plugins[0]["status"], "ready");
  assert_eq!(fixed_plugins[0]["enabled"], true);
}

#[test]
fn plugin_refresh_preserves_runtime_state_when_persistence_fails() {
  let mut context = RuntimeContext::new_in_memory();
  let storage_root = create_temp_workspace("plugin-refresh-state-warning-storage");
  let database_path = storage_root.join("amentia.db");
  fs::create_dir_all(&database_path).expect("create directory at database path");
  context
    .persistence_state
    .set_store_for_testing(RuntimeStore::new(database_path));
  let plugin_root = create_temp_plugin_bundle(
    "plugin-refresh-state-warning",
    "refresh-state",
    "Refresh State",
  );
  let scan_root = plugin_root
    .parent()
    .expect("plugin scan root")
    .to_path_buf();
  context
    .plugin_state
    .configure_roots(vec![scan_root.clone()], scan_root.clone());
  replace_plugin_catalog(
    &mut context,
    vec![bundled_plugin_entry(
      "refresh-state",
      "Refresh State",
      false,
      true,
      &["command:refresh-state.run"],
      &["file.read"],
    )],
  );

  let response = handle_request(&mut context, request(methods::PLUGIN_REFRESH, None));

  fs::remove_dir_all(&scan_root).expect("cleanup plugin root");
  fs::remove_dir_all(&storage_root).expect("cleanup storage root");

  assert!(response.error.is_none());
  let result = response.result.expect("plugin refresh result");
  assert!(
    result["stateWarning"]
      .as_str()
      .expect("state warning")
      .len()
      > 10
  );
  let plugins = result["plugins"].as_array().expect("plugin list");
  assert_eq!(plugins.len(), 1);
  assert_eq!(plugins[0]["id"], "refresh-state");
  assert_eq!(plugins[0]["enabled"], false);
}

#[test]
fn plugin_install_rejects_duplicate_plugin_ids() {
  let mut context = RuntimeContext::new_in_memory();
  let source_root = create_temp_plugin_bundle(
    "plugin-install-duplicate",
    "workspace-notes",
    "Workspace Notes",
  );
  let source_path = source_root.display().to_string();
  replace_plugin_catalog(
    &mut context,
    vec![bundled_plugin_entry(
      "workspace-notes",
      "Workspace Notes",
      true,
      true,
      &["skill:workspace.notes"],
      &["file.read"],
    )],
  );

  let response = handle_request(
    &mut context,
    request(
      methods::PLUGIN_INSTALL,
      Some(json!({
        "sourcePath": source_path.clone()
      })),
    ),
  );

  fs::remove_dir_all(source_root.parent().expect("plugin source root"))
    .expect("cleanup plugin source root");

  assert!(response.result.is_none());
  let error = response.error.expect("plugin install error");
  assert_eq!(error.code, -32053);
  assert!(error.message.contains("already installed"));
  let data = error.data.expect("plugin install error data");
  assert_eq!(data["sourcePath"], source_path.as_str());
  assert_eq!(data["pluginInstallStatus"], "alreadyInstalled");
  assert!(data["installBlocker"]
    .as_str()
    .expect("install blocker")
    .contains("already installed"));
  assert!(data["installRepairHint"]
    .as_str()
    .expect("install repair hint")
    .contains("Remove the existing local plugin first"));
}

#[test]
fn plugin_remove_deletes_local_plugin_and_clears_persisted_state() {
  let mut context = RuntimeContext::new_in_memory();
  let storage_root = create_temp_workspace("plugin-remove-storage");
  let source_root =
    create_temp_plugin_bundle("plugin-remove-source", "focus-review", "Focus Review");
  let install_root = create_temp_workspace("plugin-remove-root");
  let store = RuntimeStore::new(storage_root.join("amentia.db"));
  store
    .save_plugin_enabled("focus-review", true)
    .expect("save persisted plugin state");
  context.persistence_state.set_store_for_testing(store);
  context
    .plugin_state
    .configure_roots(vec![install_root.clone()], install_root.clone());
  replace_plugin_catalog(&mut context, vec![]);

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

  let manifest_path = context.plugin_state.catalog()[0].manifest_path.clone();
  let remove_response = handle_request(
    &mut context,
    request(
      methods::PLUGIN_REMOVE,
      Some(json!({
        "manifestPath": manifest_path
      })),
    ),
  );

  let persisted_states = context
    .persistence_state
    .store()
    .expect("store")
    .load_plugin_states()
    .expect("load plugin states");

  fs::remove_dir_all(source_root.parent().expect("plugin source root"))
    .expect("cleanup plugin source root");
  fs::remove_dir_all(&install_root).expect("cleanup install root");
  fs::remove_dir_all(&storage_root).expect("cleanup storage root");

  assert!(remove_response.error.is_none());
  let result = remove_response.result.expect("plugin remove result");
  assert_eq!(result["pluginId"], "focus-review");
  assert!(context.plugin_state.catalog().is_empty());
  assert!(!persisted_states.contains_key("focus-review"));
}

#[test]
fn plugin_remove_deletes_connector_credentials_for_local_plugin() {
  let mut context = RuntimeContext::new_in_memory();
  let storage_root = create_temp_workspace("plugin-remove-connector-storage");
  let source_root = create_temp_plugin_bundle(
    "plugin-remove-connector-source",
    "notion-remove",
    "Notion Remove",
  );
  let install_root = create_temp_workspace("plugin-remove-connector-root");
  fs::write(
    source_root.join("amentia-plugin.json"),
    r#"{
  "name": "notion-remove",
  "version": "0.1.0",
  "displayName": "Notion Remove",
  "description": "Connector removal test plugin",
  "author": { "name": "Amentia" },
  "capabilities": ["connector:notion"],
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
  "defaultEnabled": true
}"#,
  )
  .expect("write connector plugin manifest");
  context
    .persistence_state
    .set_store_for_testing(RuntimeStore::new(storage_root.join("amentia.db")));
  context
    .plugin_state
    .configure_roots(vec![install_root.clone()], install_root.clone());
  replace_plugin_catalog(&mut context, vec![]);

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

  let authorize_response = handle_request(
    &mut context,
    request(
      methods::PLUGIN_CONNECTOR_AUTHORIZE,
      Some(json!({
        "connectorId": "notion-remove::notion",
        "credentialSecret": "remove-token"
      })),
    ),
  );
  assert!(authorize_response.error.is_none());
  assert!(context
    .plugin_state
    .connector_credential("notion-remove::notion")
    .is_some());
  assert_eq!(
    context
      .persistence_state
      .store()
      .expect("store")
      .load_plugin_connector_credentials()
      .expect("load connector credentials")
      .len(),
    1
  );

  let manifest_path = context.plugin_state.catalog()[0].manifest_path.clone();
  let remove_response = handle_request(
    &mut context,
    request(
      methods::PLUGIN_REMOVE,
      Some(json!({
        "manifestPath": manifest_path
      })),
    ),
  );
  let persisted_credentials = context
    .persistence_state
    .store()
    .expect("store")
    .load_plugin_connector_credentials()
    .expect("load connector credentials after removal");

  fs::remove_dir_all(source_root.parent().expect("plugin source root"))
    .expect("cleanup plugin source root");
  fs::remove_dir_all(&install_root).expect("cleanup install root");
  fs::remove_dir_all(&storage_root).expect("cleanup storage root");

  assert!(remove_response.error.is_none());
  assert!(context.plugin_state.catalog().is_empty());
  assert!(context
    .plugin_state
    .connector_credential("notion-remove::notion")
    .is_none());
  assert!(persisted_credentials.is_empty());
}

#[test]
fn plugin_remove_refreshes_catalog_after_persistence_cleanup_fails() {
  let mut context = RuntimeContext::new_in_memory();
  let storage_root = create_temp_workspace("plugin-remove-failing-storage");
  let database_path = storage_root.join("amentia.db");
  let source_root = create_temp_plugin_bundle(
    "plugin-remove-refresh-source",
    "focus-review",
    "Focus Review",
  );
  let install_root = create_temp_workspace("plugin-remove-refresh-root");
  fs::create_dir_all(&database_path).expect("create directory at database path");
  context
    .plugin_state
    .configure_roots(vec![install_root.clone()], install_root.clone());
  replace_plugin_catalog(&mut context, vec![]);

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

  context
    .persistence_state
    .set_store_for_testing(RuntimeStore::new(database_path));
  let manifest_path = context.plugin_state.catalog()[0].manifest_path.clone();
  let remove_response = handle_request(
    &mut context,
    request(
      methods::PLUGIN_REMOVE,
      Some(json!({
        "manifestPath": manifest_path
      })),
    ),
  );

  assert!(!std::path::Path::new(&manifest_path).exists());
  assert!(context.plugin_state.catalog().is_empty());

  fs::remove_dir_all(source_root.parent().expect("plugin source root"))
    .expect("cleanup plugin source root");
  fs::remove_dir_all(&install_root).expect("cleanup install root");
  fs::remove_dir_all(&storage_root).expect("cleanup storage root");

  assert!(remove_response.result.is_none());
  let error = remove_response.error.expect("plugin remove error");
  assert_eq!(error.code, -32010);
  let data = error.data.expect("plugin remove error data");
  assert_eq!(data["pluginId"], "focus-review");
  assert_eq!(data["pluginLifecycleOperation"], "remove");
  assert_eq!(data["pluginLifecycleStatus"], "cleanupFailed");
  assert_eq!(data["sourcePath"], manifest_path);
  assert!(
    data["lifecycleBlocker"]
      .as_str()
      .expect("lifecycle blocker")
      .len()
      > 10
  );
  assert!(data["lifecycleRepairHint"]
    .as_str()
    .expect("lifecycle repair hint")
    .contains("storage permissions"));
}
