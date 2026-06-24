use super::tests_support::create_temp_plugin_root;
use super::*;
use std::fs;
use std::path::PathBuf;

#[test]
fn discover_plugins_in_roots_merges_local_and_bundled_catalogs() {
  let bundled_root = create_temp_plugin_root("discover-multi-bundled");
  let local_root = create_temp_plugin_root("discover-multi-local");
  let bundled_plugin_dir = bundled_root.join("bundled").join("workspace-notes");
  let local_plugin_dir = local_root.join("focus-review");
  fs::create_dir_all(&bundled_plugin_dir).expect("create bundled plugin dir");
  fs::create_dir_all(bundled_plugin_dir.join("skills")).expect("create bundled skills dir");
  fs::create_dir_all(&local_plugin_dir).expect("create local plugin dir");
  fs::write(
    bundled_plugin_dir.join("amentia-plugin.json"),
    r#"{
  "name": "workspace-notes",
  "version": "0.1.0",
  "displayName": "Workspace Notes",
  "description": "Bundled plugin",
  "author": { "name": "Amentia" },
  "capabilities": ["skill:workspace.notes"],
  "skills": [
    {
      "id": "workspace.notes",
      "description": "Use workspace notes.",
      "path": "skills/workspace-notes.md"
    }
  ],
  "permissions": ["file.read"],
  "defaultEnabled": true
}"#,
  )
  .expect("write bundled manifest");
  fs::write(
    bundled_plugin_dir.join("skills/workspace-notes.md"),
    "Use this skill to keep workspace notes concise.",
  )
  .expect("write bundled skill file");
  fs::write(
    local_plugin_dir.join("amentia-plugin.json"),
    r#"{
  "name": "focus-review",
  "version": "0.1.0",
  "displayName": "Focus Review",
  "description": "Local plugin",
  "author": { "name": "Amentia" },
  "capabilities": ["command:review.focus"],
  "permissions": ["file.read"],
  "defaultEnabled": false
}"#,
  )
  .expect("write local manifest");

  let plugins = discover_plugins_in_roots(&[bundled_root.clone(), local_root.clone()])
    .expect("discover plugins across roots");

  fs::remove_dir_all(&bundled_root).expect("cleanup bundled plugin root");
  fs::remove_dir_all(&local_root).expect("cleanup local plugin root");

  assert_eq!(plugins.len(), 2);
  assert!(plugins.iter().any(|plugin| plugin.id == "workspace-notes"));
  assert!(plugins.iter().any(|plugin| plugin.id == "focus-review"));
}

#[cfg(unix)]
#[test]
fn discover_plugins_skips_symlinked_plugin_directories() {
  use std::os::unix::fs::symlink;

  let local_root = create_temp_plugin_root("discover-symlink-local");
  let outside_root = create_temp_plugin_root("discover-symlink-outside");
  let outside_plugin_dir = outside_root.join("outside-plugin");
  fs::create_dir_all(&outside_plugin_dir).expect("create outside plugin dir");
  fs::write(
    outside_plugin_dir.join("amentia-plugin.json"),
    r#"{
  "name": "outside-plugin",
  "version": "0.1.0",
  "displayName": "Outside Plugin",
  "description": "Plugin outside the configured root.",
  "author": { "name": "Amentia" },
  "capabilities": ["command:outside.run"],
  "permissions": ["file.read"],
  "defaultEnabled": true
}"#,
  )
  .expect("write outside manifest");
  symlink(&outside_plugin_dir, local_root.join("outside-plugin"))
    .expect("create plugin dir symlink");

  let plugins = discover_plugins(&local_root).expect("discover plugins");

  fs::remove_dir_all(&local_root).expect("cleanup local plugin root");
  fs::remove_dir_all(&outside_root).expect("cleanup outside plugin root");

  assert!(plugins.is_empty());
}

#[test]
fn install_and_remove_local_plugin_bundle_round_trip() {
  let source_root = create_temp_plugin_root("install-source");
  let install_root = create_temp_plugin_root("install-target");
  let source_plugin_dir = source_root.join("focus-review");
  fs::create_dir_all(source_plugin_dir.join("commands")).expect("create source commands dir");
  fs::write(
    source_plugin_dir.join("amentia-plugin.json"),
    r#"{
  "name": "focus-review",
  "version": "0.1.0",
  "displayName": "Focus Review",
  "description": "Local plugin",
  "author": { "name": "Amentia" },
  "capabilities": ["command:review.focus"],
  "permissions": ["file.read"],
  "defaultEnabled": true
}"#,
  )
  .expect("write source manifest");
  fs::write(
    source_plugin_dir.join("commands/review.focus.json"),
    r#"{
  "title": "Focus Review",
  "description": "Prepare a focused review summary.",
  "prompt": "Review the latest diff and keep the summary focused."
}"#,
  )
  .expect("write command definition");

  let installed_plugin =
    install_plugin_bundle(&source_plugin_dir, &install_root).expect("install plugin bundle");
  let installed_manifest = PathBuf::from(&installed_plugin.manifest_path);
  let removed_plugin =
    remove_local_plugin_bundle(&installed_manifest, &install_root).expect("remove plugin");

  fs::remove_dir_all(&source_root).expect("cleanup source root");
  fs::remove_dir_all(&install_root).expect("cleanup install root");

  assert_eq!(installed_plugin.id, "focus-review");
  assert_eq!(installed_plugin.provenance, "local");
  assert_eq!(removed_plugin.plugin_id, "focus-review");
  assert!(removed_plugin.removed_path.ends_with("focus-review"));
}

#[test]
fn inspect_rejects_nested_plugin_manifests() {
  let source_root = create_temp_plugin_root("install-nested-manifest-source");
  let source_plugin_dir = source_root.join("focus-review");
  let nested_plugin_dir = source_plugin_dir.join("nested");
  fs::create_dir_all(&nested_plugin_dir).expect("create nested plugin dir");
  fs::write(
    source_plugin_dir.join("amentia-plugin.json"),
    r#"{
  "name": "focus-review",
  "version": "0.1.0",
  "displayName": "Focus Review",
  "description": "Local plugin",
  "author": { "name": "Amentia" },
  "capabilities": ["command:review.focus"],
  "permissions": ["file.read"],
  "defaultEnabled": true
}"#,
  )
  .expect("write source manifest");
  fs::write(
    nested_plugin_dir.join("amentia-plugin.json"),
    r#"{
  "name": "hidden-plugin",
  "version": "0.1.0",
  "displayName": "Hidden Plugin",
  "description": "Nested plugin",
  "author": { "name": "Amentia" },
  "capabilities": ["command:hidden.run"],
  "permissions": ["file.read"],
  "defaultEnabled": true
}"#,
  )
  .expect("write nested manifest");

  let error = inspect_plugin_bundle(&source_plugin_dir)
    .expect_err("nested plugin manifests should be rejected");

  fs::remove_dir_all(&source_root).expect("cleanup source root");

  assert!(error.to_string().contains("nested amentia-plugin.json"));
}

#[test]
fn inspect_reports_manifest_validation_hint() {
  let source_root = create_temp_plugin_root("inspect-invalid-manifest-source");
  let source_plugin_dir = source_root.join("invalid-plugin");
  fs::create_dir_all(&source_plugin_dir).expect("create source plugin dir");
  fs::write(
    source_plugin_dir.join("amentia-plugin.json"),
    r#"{
  "name": "invalid-plugin",
  "version": "0.1.0",
  "displayName": "Invalid Plugin",
  "description": "Invalid local plugin",
  "author": { "name": "Amentia" },
  "capabilities": ["command:invalid.run"],
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
    "credentialStore": "keychain"
  },
  "defaultEnabled": true
}"#,
  )
  .expect("write invalid manifest");

  let error =
    inspect_plugin_bundle(&source_plugin_dir).expect_err("invalid plugin should be rejected");

  fs::remove_dir_all(&source_root).expect("cleanup source root");

  let message = error.to_string();
  assert!(message.contains("plugin credential store `keychain` is not supported"));
  assert!(message.contains("Hint:"));
  assert!(message.contains("supported credential stores"));
}

#[test]
fn remove_rejects_manifest_at_install_root() {
  let install_root = create_temp_plugin_root("remove-install-root-manifest");
  fs::write(
    install_root.join("amentia-plugin.json"),
    r#"{
  "name": "root-plugin",
  "version": "0.1.0",
  "displayName": "Root Plugin",
  "description": "Invalid install root plugin",
  "author": { "name": "Amentia" },
  "capabilities": ["command:root.run"],
  "permissions": ["file.read"],
  "defaultEnabled": true
}"#,
  )
  .expect("write root manifest");
  fs::create_dir_all(install_root.join("sibling")).expect("create sibling dir");

  let error = remove_local_plugin_bundle(&install_root.join("amentia-plugin.json"), &install_root)
    .expect_err("install root manifest should not be removable as a plugin");

  assert!(install_root.exists());
  assert!(install_root.join("sibling").exists());

  fs::remove_dir_all(&install_root).expect("cleanup install root");

  assert!(error.to_string().contains("directly under"));
}

#[cfg(unix)]
#[test]
fn install_rejects_plugin_bundle_symlinks_and_cleans_destination() {
  use std::os::unix::fs::symlink;

  let source_root = create_temp_plugin_root("install-symlink-source");
  let install_root = create_temp_plugin_root("install-symlink-target");
  let outside_root = create_temp_plugin_root("install-symlink-outside");
  let source_plugin_dir = source_root.join("symlink-plugin");
  fs::create_dir_all(source_plugin_dir.join("commands")).expect("create source commands dir");
  fs::write(
    source_plugin_dir.join("amentia-plugin.json"),
    r#"{
  "name": "symlink-plugin",
  "version": "0.1.0",
  "displayName": "Symlink Plugin",
  "description": "Local plugin with a symlink.",
  "author": { "name": "Amentia" },
  "capabilities": ["command:review.focus"],
  "permissions": ["file.read"],
  "defaultEnabled": true
}"#,
  )
  .expect("write source manifest");
  fs::write(outside_root.join("outside.txt"), "outside").expect("write outside file");
  symlink(
    outside_root.join("outside.txt"),
    source_plugin_dir.join("commands/outside-link.json"),
  )
  .expect("create plugin symlink");

  let error = install_plugin_bundle(&source_plugin_dir, &install_root)
    .expect_err("symlinked plugin bundle should be rejected");

  assert!(!install_root.join("symlink-plugin").exists());

  fs::remove_dir_all(&source_root).expect("cleanup source root");
  fs::remove_dir_all(&install_root).expect("cleanup install root");
  fs::remove_dir_all(&outside_root).expect("cleanup outside root");

  assert!(error.to_string().contains("symbolic links"));
}
