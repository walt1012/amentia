use super::tests_support::create_temp_plugin_root;
use super::*;
use std::fs;

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
  let execution = commands[0]
    .execution
    .as_ref()
    .expect("command execution contract");
  assert_eq!(execution.kind, "builtin.workspaceReadmeNote");
  assert_eq!(execution.driver, "builtin");
  assert_eq!(execution.entrypoint, None);
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
