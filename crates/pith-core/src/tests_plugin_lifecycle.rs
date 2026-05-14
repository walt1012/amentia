use super::test_support::{
  bundled_plugin_entry, create_temp_plugin_bundle, create_temp_workspace, replace_plugin_catalog,
  request,
};
use super::*;
use pith_protocol::methods;
use pith_storage::RuntimeStore;
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
      &["prompt_pack:workspace.notes"],
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
  let database_path = storage_root.join("pith.db");
  fs::create_dir_all(&database_path).expect("create directory at database path");
  context
    .persistence_state
    .set_store_for_testing(RuntimeStore::new(
      database_path,
      storage_root.join("threads.json"),
    ));
  replace_plugin_catalog(
    &mut context,
    vec![bundled_plugin_entry(
      "workspace-notes",
      "Workspace Notes",
      false,
      false,
      &["prompt_pack:workspace.notes"],
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
  assert!(context.plugin_state.catalog().is_empty());
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
fn plugin_install_rejects_duplicate_plugin_ids() {
  let mut context = RuntimeContext::new_in_memory();
  let source_root = create_temp_plugin_bundle(
    "plugin-install-duplicate",
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
      &["prompt_pack:workspace.notes"],
      &["file.read"],
    )],
  );

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

  assert!(response.result.is_none());
  let error = response.error.expect("plugin install error");
  assert!(error.message.contains("already installed"));
}

#[test]
fn plugin_remove_deletes_local_plugin_and_clears_persisted_state() {
  let mut context = RuntimeContext::new_in_memory();
  let storage_root = create_temp_workspace("plugin-remove-storage");
  let source_root =
    create_temp_plugin_bundle("plugin-remove-source", "focus-review", "Focus Review");
  let install_root = create_temp_workspace("plugin-remove-root");
  let store = RuntimeStore::new(
    storage_root.join("pith.db"),
    storage_root.join("threads.json"),
  );
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
fn plugin_remove_refreshes_catalog_after_persistence_cleanup_fails() {
  let mut context = RuntimeContext::new_in_memory();
  let storage_root = create_temp_workspace("plugin-remove-failing-storage");
  let database_path = storage_root.join("pith.db");
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
    .set_store_for_testing(RuntimeStore::new(
      database_path,
      storage_root.join("threads.json"),
    ));
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
}
