mod catalog;
mod io;
mod lifecycle;
mod manifest;
mod paths;
mod registry;
mod types;
mod validation;

pub use catalog::{discover_plugins, discover_plugins_in_roots};
pub use lifecycle::{inspect_plugin_bundle, install_plugin_bundle, remove_local_plugin_bundle};
pub use manifest::{
  PluginAppConnectorManifest, PluginAuthPolicyManifest, PluginAuthor, PluginManifest,
  PluginMcpServerManifest, PluginSkillManifest,
};
pub use paths::{configured_plugin_install_root, configured_plugin_roots, default_plugin_root};
pub use registry::{
  build_capability_registry, build_command_registry, build_connector_registry, build_hook_registry,
};
pub use types::{
  PluginCapabilityRegistration, PluginCatalogEntry, PluginCommandEntry, PluginConnectorEntry,
  PluginHookEntry, PluginRemovalRecord,
};

#[cfg(test)]
mod tests {
  use super::validation::{manifest_capabilities, validate_manifest, validation_hint_for_error};
  use super::*;
  use std::env;
  use std::fs;
  use std::path::PathBuf;
  use std::time::{SystemTime, UNIX_EPOCH};

  fn create_temp_plugin_root(label: &str) -> PathBuf {
    let unique = SystemTime::now()
      .duration_since(UNIX_EPOCH)
      .expect("system time")
      .as_nanos();
    let path = env::temp_dir().join(format!("pith-plugin-host-{label}-{unique}"));
    fs::create_dir_all(&path).expect("create temp plugin root");
    path
  }

  fn manifest(capabilities: Vec<&str>, permissions: Vec<&str>) -> PluginManifest {
    PluginManifest {
      name: "workspace-notes".to_string(),
      version: "0.1.0".to_string(),
      display_name: "Workspace Notes".to_string(),
      description: "Test plugin".to_string(),
      author: Some(PluginAuthor {
        name: "Pith".to_string(),
      }),
      capabilities: capabilities.into_iter().map(str::to_string).collect(),
      permissions: permissions.into_iter().map(str::to_string).collect(),
      skills: vec![],
      mcp_servers: vec![],
      app_connectors: vec![],
      auth_policy: None,
      default_enabled: true,
    }
  }

  #[test]
  fn validate_manifest_accepts_typed_capabilities_and_permissions() {
    let mut manifest = manifest(
      vec![
        "prompt_pack:workspace.notes",
        "settings:workspace.preferences",
      ],
      vec!["file.read", "file.write"],
    );
    manifest.skills = vec![PluginSkillManifest {
      id: "workspace.notes".to_string(),
      description: "Workspace note guidance.".to_string(),
      path: "skills/workspace-notes.md".to_string(),
    }];
    manifest.mcp_servers = vec![PluginMcpServerManifest {
      id: "workspace.mcp".to_string(),
      command: Some("pith-workspace-mcp".to_string()),
      args: vec![],
      transport: Some("stdio".to_string()),
    }];
    manifest.app_connectors = vec![PluginAppConnectorManifest {
      id: "workspace.connector".to_string(),
      display_name: "Workspace Connector".to_string(),
      service: "workspace".to_string(),
      homepage: None,
    }];
    manifest.auth_policy = Some(PluginAuthPolicyManifest {
      auth_type: "none".to_string(),
      required: false,
      scopes: vec![],
      credential_store: Some("none".to_string()),
    });

    let result = validate_manifest(&manifest);

    assert!(result.is_ok());
    let capabilities = manifest_capabilities(&manifest);
    assert!(capabilities
      .iter()
      .any(|capability| capability == "skill:workspace.notes"));
    assert!(capabilities
      .iter()
      .any(|capability| capability == "mcp_server:workspace.mcp"));
    assert!(capabilities
      .iter()
      .any(|capability| capability == "connector:workspace.connector"));
  }

  #[test]
  fn validate_manifest_rejects_legacy_capability_format() {
    let manifest = manifest(vec!["memory.notes"], vec!["file.read"]);

    let error = validate_manifest(&manifest).expect_err("legacy capability should fail");

    assert!(error
      .to_string()
      .contains("must use the `<kind>:<identifier>` format"));
  }

  #[test]
  fn validate_manifest_rejects_plugin_name_path_segments() {
    let mut manifest = manifest(vec!["command:review.focus"], vec!["file.read"]);
    manifest.name = "../focus-review".to_string();

    let error = validate_manifest(&manifest).expect_err("path segment plugin name should fail");

    assert!(error
      .to_string()
      .contains("must not contain path separators"));
  }

  #[test]
  fn build_capability_registry_skips_disabled_and_invalid_plugins() {
    let plugins = vec![
      PluginCatalogEntry {
        id: "workspace-notes".to_string(),
        name: "workspace-notes".to_string(),
        version: "0.1.0".to_string(),
        display_name: "Workspace Notes".to_string(),
        status: "ready".to_string(),
        description: "Enabled plugin".to_string(),
        author_name: Some("Pith".to_string()),
        enabled: true,
        default_enabled: true,
        capabilities: vec![
          "prompt_pack:workspace.notes".to_string(),
          "settings:workspace.preferences".to_string(),
        ],
        permissions: vec!["file.read".to_string(), "file.write".to_string()],
        manifest_path: "plugins/bundled/workspace-notes/pith-plugin.json".to_string(),
        provenance: "bundled".to_string(),
        validation_error: None,
        validation_hint: None,
      },
      PluginCatalogEntry {
        id: "shell-recorder".to_string(),
        name: "shell-recorder".to_string(),
        version: "0.1.0".to_string(),
        display_name: "Shell Recorder".to_string(),
        status: "ready".to_string(),
        description: "Disabled plugin".to_string(),
        author_name: Some("Pith".to_string()),
        enabled: false,
        default_enabled: false,
        capabilities: vec!["hook:shell.recorder".to_string()],
        permissions: vec!["shell.exec".to_string()],
        manifest_path: "plugins/bundled/shell-recorder/pith-plugin.json".to_string(),
        provenance: "bundled".to_string(),
        validation_error: None,
        validation_hint: None,
      },
      PluginCatalogEntry {
        id: "broken-plugin".to_string(),
        name: "broken-plugin".to_string(),
        version: "invalid".to_string(),
        display_name: "Broken Plugin".to_string(),
        status: "invalid".to_string(),
        description: "Invalid plugin".to_string(),
        author_name: None,
        enabled: false,
        default_enabled: false,
        capabilities: vec![],
        permissions: vec![],
        manifest_path: "plugins/bundled/broken/pith-plugin.json".to_string(),
        provenance: "bundled".to_string(),
        validation_error: Some("plugin capability kind `memory` is not supported".to_string()),
        validation_hint: Some(
          "Use one of the supported capability kinds: command, agent, prompt_pack, hook, tool, mcp_server, skill, connector, settings."
            .to_string(),
        ),
      },
    ];

    let registry = build_capability_registry(&plugins);

    assert_eq!(registry.len(), 2);
    assert_eq!(registry[0].kind, "prompt_pack");
    assert_eq!(registry[1].kind, "settings");
    assert!(registry
      .iter()
      .all(|entry| entry.plugin_id == "workspace-notes"));
  }

  #[test]
  fn build_capability_registry_includes_connector_metadata() {
    let plugin_root = create_temp_plugin_root("connector-metadata");
    let plugin_dir = plugin_root.join("notion-connector");
    fs::create_dir_all(&plugin_dir).expect("create connector plugin dir");
    fs::write(
      plugin_dir.join("pith-plugin.json"),
      r#"{
  "name": "notion-connector",
  "version": "0.1.0",
  "displayName": "Notion Connector",
  "description": "Connector plugin",
  "author": { "name": "Pith" },
  "capabilities": [],
  "permissions": ["network.outbound", "mcp.connect"],
  "mcpServers": [{ "id": "notion", "transport": "stdio" }],
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
    "scopes": ["read_content", "insert_content"],
    "credentialStore": "keychain"
  },
  "defaultEnabled": true
}"#,
    )
    .expect("write connector manifest");
    let plugins =
      discover_plugins_in_roots(std::slice::from_ref(&plugin_root)).expect("discover connector");

    let registry = build_capability_registry(&plugins);

    fs::remove_dir_all(&plugin_root).expect("cleanup connector plugin root");

    let connector = registry
      .iter()
      .find(|capability| capability.capability_id == "notion-connector::connector:notion")
      .expect("connector capability");
    assert_eq!(connector.metadata["service"], "notion");
    assert_eq!(connector.metadata["authType"], "oauth2");
    assert_eq!(connector.metadata["credentialStore"], "keychain");
    assert_eq!(
      connector.metadata["authScopes"],
      "read_content, insert_content"
    );

    let mcp_server = registry
      .iter()
      .find(|capability| capability.capability_id == "notion-connector::mcp_server:notion")
      .expect("mcp server capability");
    assert_eq!(mcp_server.metadata["transport"], "stdio");
  }

  #[test]
  fn build_connector_registry_lists_disabled_third_party_connectors() {
    let plugin_root = create_temp_plugin_root("connector-registry");
    let plugin_dir = plugin_root.join("notion-connector");
    fs::create_dir_all(&plugin_dir).expect("create connector plugin dir");
    fs::write(
      plugin_dir.join("pith-plugin.json"),
      r#"{
  "name": "notion-connector",
  "version": "0.1.0",
  "displayName": "Notion Connector",
  "description": "Connector plugin",
  "author": { "name": "Pith" },
  "capabilities": [],
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
    "scopes": ["read_content", "insert_content"],
    "credentialStore": "keychain"
  },
  "defaultEnabled": false
}"#,
    )
    .expect("write connector manifest");
    let plugins =
      discover_plugins_in_roots(std::slice::from_ref(&plugin_root)).expect("discover connector");

    let connectors = build_connector_registry(&plugins);

    fs::remove_dir_all(&plugin_root).expect("cleanup connector plugin root");

    assert_eq!(connectors.len(), 1);
    assert_eq!(connectors[0].connector_id, "notion-connector::notion");
    assert_eq!(connectors[0].status, "disabled");
    assert!(!connectors[0].enabled);
    assert_eq!(connectors[0].auth_type.as_deref(), Some("oauth2"));
    assert!(connectors[0].auth_required);
    assert_eq!(
      connectors[0].auth_scopes,
      vec!["read_content".to_string(), "insert_content".to_string()]
    );
    assert_eq!(connectors[0].credential_store.as_deref(), Some("keychain"));
  }

  #[test]
  fn build_command_registry_loads_enabled_plugin_commands() {
    let plugin_root = create_temp_plugin_root("command-registry");
    let plugin_dir = plugin_root.join("workspace-notes");
    let commands_dir = plugin_dir.join("commands");
    fs::create_dir_all(&commands_dir).expect("create commands dir");
    fs::write(
      plugin_dir.join("pith-plugin.json"),
      r#"{
  "name": "workspace-notes",
  "version": "0.1.0",
  "displayName": "Workspace Notes",
  "description": "Test plugin",
  "author": { "name": "Pith" },
  "capabilities": [
    "command:workspace.capture-note",
    "prompt_pack:workspace.notes"
  ],
  "permissions": [
    "file.read",
    "file.write"
  ],
  "defaultEnabled": true
}"#,
    )
    .expect("write plugin manifest");
    fs::write(
      commands_dir.join("workspace.capture-note.json"),
      r#"{
  "title": "Capture Workspace Note",
  "description": "Prepare a reusable note from the current workspace.",
  "prompt": "Read README.md and summarize the most reusable workspace detail.",
  "execution": {
    "kind": "builtin.workspaceReadmeNote"
  },
  "memory": {
    "noteTitle": "Workspace Capture",
    "noteSource": "plugin.workspace-notes",
    "noteTags": ["plugin", "workspace"]
  }
}"#,
    )
    .expect("write command definition");

    let plugins = discover_plugins(&plugin_root).expect("discover plugins");
    let commands = build_command_registry(&plugins);

    fs::remove_dir_all(&plugin_root).expect("cleanup plugin root");

    assert_eq!(commands.len(), 1);
    assert_eq!(commands[0].plugin_id, "workspace-notes");
    assert_eq!(commands[0].title, "Capture Workspace Note");
    assert_eq!(
      commands[0].memory_note_title.as_deref(),
      Some("Workspace Capture")
    );
    assert_eq!(
      commands[0].execution_kind.as_deref(),
      Some("builtin.workspaceReadmeNote")
    );
    assert_eq!(
      commands[0].memory_note_source.as_deref(),
      Some("plugin.workspace-notes")
    );
    assert_eq!(
      commands[0].memory_note_tags,
      vec!["plugin".to_string(), "workspace".to_string()]
    );
    assert!(commands[0]
      .source_path
      .ends_with("workspace.capture-note.json"));
  }

  #[test]
  fn build_hook_registry_loads_enabled_plugin_hooks() {
    let plugin_root = create_temp_plugin_root("hook-registry");
    let plugin_dir = plugin_root.join("shell-recorder");
    let hooks_dir = plugin_dir.join("hooks");
    fs::create_dir_all(&hooks_dir).expect("create hooks dir");
    fs::write(
      plugin_dir.join("pith-plugin.json"),
      r#"{
  "name": "shell-recorder",
  "version": "0.1.0",
  "displayName": "Shell Recorder",
  "description": "Test plugin",
  "author": { "name": "Pith" },
  "capabilities": [
    "hook:shell.recorder",
    "tool:shell.timeline"
  ],
  "permissions": [
    "shell.exec"
  ],
  "defaultEnabled": true
}"#,
    )
    .expect("write plugin manifest");
    fs::write(
      hooks_dir.join("shell.recorder.json"),
      r#"{
  "title": "Record Shell Completion",
  "description": "Capture a compact shell completion note in the thread timeline.",
  "event": "shell.completed",
  "messageTemplate": "Hook observed {{command}} in {{workspaceName}}.",
  "memory": {
    "noteTitle": "Shell Completion",
    "noteSource": "plugin.shell-recorder",
    "noteTags": ["shell", "hook"]
  }
}"#,
    )
    .expect("write hook definition");

    let plugins = discover_plugins(&plugin_root).expect("discover plugins");
    let hooks = build_hook_registry(&plugins);

    fs::remove_dir_all(&plugin_root).expect("cleanup plugin root");

    assert_eq!(hooks.len(), 1);
    assert_eq!(hooks[0].plugin_id, "shell-recorder");
    assert_eq!(hooks[0].event, "shell.completed");
    assert_eq!(
      hooks[0].memory_note_title.as_deref(),
      Some("Shell Completion")
    );
    assert_eq!(
      hooks[0].memory_note_source.as_deref(),
      Some("plugin.shell-recorder")
    );
    assert_eq!(
      hooks[0].memory_note_tags,
      vec!["shell".to_string(), "hook".to_string()]
    );
    assert!(hooks[0].source_path.ends_with("shell.recorder.json"));
  }

  #[test]
  fn discover_plugins_in_roots_merges_local_and_bundled_catalogs() {
    let bundled_root = create_temp_plugin_root("discover-multi-bundled");
    let local_root = create_temp_plugin_root("discover-multi-local");
    let bundled_plugin_dir = bundled_root.join("bundled").join("workspace-notes");
    let local_plugin_dir = local_root.join("focus-review");
    fs::create_dir_all(&bundled_plugin_dir).expect("create bundled plugin dir");
    fs::create_dir_all(&local_plugin_dir).expect("create local plugin dir");
    fs::write(
      bundled_plugin_dir.join("pith-plugin.json"),
      r#"{
  "name": "workspace-notes",
  "version": "0.1.0",
  "displayName": "Workspace Notes",
  "description": "Bundled plugin",
  "author": { "name": "Pith" },
  "capabilities": ["prompt_pack:workspace.notes"],
  "permissions": ["file.read"],
  "defaultEnabled": true
}"#,
    )
    .expect("write bundled manifest");
    fs::write(
      local_plugin_dir.join("pith-plugin.json"),
      r#"{
  "name": "focus-review",
  "version": "0.1.0",
  "displayName": "Focus Review",
  "description": "Local plugin",
  "author": { "name": "Pith" },
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

  #[test]
  fn install_and_remove_local_plugin_bundle_round_trip() {
    let source_root = create_temp_plugin_root("install-source");
    let install_root = create_temp_plugin_root("install-target");
    let source_plugin_dir = source_root.join("focus-review");
    fs::create_dir_all(source_plugin_dir.join("commands")).expect("create source commands dir");
    fs::write(
      source_plugin_dir.join("pith-plugin.json"),
      r#"{
  "name": "focus-review",
  "version": "0.1.0",
  "displayName": "Focus Review",
  "description": "Local plugin",
  "author": { "name": "Pith" },
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
  fn bundled_plugin_manifests_match_runtime_schema() {
    let bundled_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../plugins/bundled");
    let manifests = [
      bundled_root.join("workspace-notes/pith-plugin.json"),
      bundled_root.join("shell-recorder/pith-plugin.json"),
      bundled_root.join("review-assistant/pith-plugin.json"),
      bundled_root.join("notion-connector/pith-plugin.json"),
    ];

    for manifest_path in manifests {
      let manifest = read_manifest(&manifest_path).expect("parse bundled manifest");
      validate_manifest(&manifest).expect("validate bundled manifest");
      assert!(!manifest.display_name.trim().is_empty());
    }

    let workspace_command = super::io::read_command_manifest(
      &bundled_root.join("workspace-notes/commands/workspace.capture-note.json"),
    )
    .expect("parse workspace command manifest");
    assert_eq!(workspace_command.title, "Capture Workspace Note");
    assert_eq!(
      workspace_command
        .execution
        .as_ref()
        .map(|execution| execution.kind.as_str()),
      Some("builtin.workspaceReadmeNote")
    );
    assert_eq!(
      workspace_command
        .memory
        .as_ref()
        .map(|memory| memory.note_title.as_str()),
      Some("Workspace Capture")
    );

    let shell_command = super::io::read_command_manifest(
      &bundled_root.join("shell-recorder/commands/shell.summarize-session.json"),
    )
    .expect("parse shell command manifest");
    assert_eq!(shell_command.title, "Summarize Shell Session");
    assert_eq!(
      shell_command
        .execution
        .as_ref()
        .map(|execution| execution.kind.as_str()),
      Some("builtin.shellSessionSummary")
    );

    let review_command = super::io::read_command_manifest(
      &bundled_root.join("review-assistant/commands/review.inspect-diff.json"),
    )
    .expect("parse review command manifest");
    assert_eq!(review_command.title, "Inspect Current Diff");
    assert_eq!(
      review_command
        .execution
        .as_ref()
        .map(|execution| execution.kind.as_str()),
      Some("builtin.reviewDiffSummary")
    );

    let hook_manifest =
      super::io::read_hook_manifest(&bundled_root.join("shell-recorder/hooks/shell.recorder.json"))
        .expect("parse bundled hook manifest");
    assert_eq!(hook_manifest.event, "shell.completed");
    assert!(!hook_manifest.message_template.trim().is_empty());
    assert_eq!(
      hook_manifest
        .memory
        .as_ref()
        .map(|memory| memory.note_title.as_str()),
      Some("Shell Completion")
    );

    let notion_manifest = read_manifest(&bundled_root.join("notion-connector/pith-plugin.json"))
      .expect("parse notion connector manifest");
    let notion_capabilities = manifest_capabilities(&notion_manifest);
    assert!(notion_capabilities
      .iter()
      .any(|capability| capability == "connector:notion"));
    assert!(notion_capabilities
      .iter()
      .any(|capability| capability == "mcp_server:notion"));
  }

  #[test]
  fn validation_hint_describes_supported_capability_kinds() {
    let hint = validation_hint_for_error("plugin capability kind `memory` is not supported");

    assert!(hint.contains("supported capability kinds"));
    assert!(hint.contains("command"));
    assert!(hint.contains("connector"));
    assert!(hint.contains("settings"));
  }
}
