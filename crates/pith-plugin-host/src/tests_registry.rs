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
        "command:../outside".to_string(),
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
    "credentialStore": "local"
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
  assert_eq!(connector.metadata["credentialStore"], "local");
  assert_eq!(
    connector.metadata["authScopes"],
    "read_content, insert_content"
  );

  let mcp_server = registry
    .iter()
    .find(|capability| capability.capability_id == "notion-connector::mcp_server:notion")
    .expect("mcp server capability");
  assert_eq!(mcp_server.metadata["transport"], "stdio");
  assert_eq!(mcp_server.metadata["serverStatus"], "missingCommand");
  assert!(mcp_server.metadata["serverError"].contains("requires a command"));
}

#[test]
fn build_capability_registry_reports_mcp_server_status() {
  let plugin_root = create_temp_plugin_root("mcp-server-status");
  let plugin_dir = plugin_root.join("mcp-tools");
  fs::create_dir_all(&plugin_dir).expect("create mcp plugin dir");
  fs::write(
    plugin_dir.join("pith-plugin.json"),
    r#"{
  "name": "mcp-tools",
  "version": "0.1.0",
  "displayName": "MCP Tools",
  "description": "MCP status plugin",
  "author": { "name": "Pith" },
  "capabilities": [],
  "permissions": ["mcp.connect"],
  "mcpServers": [
    {
      "id": "ready",
      "transport": "stdio",
      "command": "bin/ready-mcp",
      "args": ["--profile", "test"]
    },
    {
      "id": "missing",
      "transport": "stdio"
    }
  ],
  "defaultEnabled": true
}"#,
  )
  .expect("write mcp manifest");
  let plugins = discover_plugins(&plugin_root).expect("discover plugins");
  let registry = build_capability_registry(&plugins);

  fs::remove_dir_all(&plugin_root).expect("cleanup mcp plugin root");

  let ready = registry
    .iter()
    .find(|capability| capability.capability_id == "mcp-tools::mcp_server:ready")
    .expect("ready mcp server capability");
  assert_eq!(ready.metadata["serverStatus"], "ready");
  assert_eq!(ready.metadata["command"], "bin/ready-mcp");
  assert_eq!(ready.metadata["args"], "--profile test");

  let missing = registry
    .iter()
    .find(|capability| capability.capability_id == "mcp-tools::mcp_server:missing")
    .expect("missing mcp server capability");
  assert_eq!(missing.metadata["serverStatus"], "missingCommand");
  assert!(missing.metadata["serverError"].contains("requires a command"));
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
    "credentialStore": "local"
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
  assert_eq!(connectors[0].credential_store.as_deref(), Some("local"));
}

#[test]
fn build_capability_registry_reports_command_definition_status() {
  let plugin_root = create_temp_plugin_root("command-definition-status");
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
    "command:workspace.missing-note"
  ],
  "permissions": ["file.read"],
  "defaultEnabled": true
}"#,
  )
  .expect("write plugin manifest");
  fs::write(
    commands_dir.join("workspace.capture-note.json"),
    r#"{
  "title": "Capture Workspace Note",
  "description": "Prepare a reusable note.",
  "prompt": "Capture a note."
}"#,
  )
  .expect("write command definition");

  let plugins = discover_plugins(&plugin_root).expect("discover plugins");
  let registry = build_capability_registry(&plugins);

  fs::remove_dir_all(&plugin_root).expect("cleanup plugin root");

  let ready = registry
    .iter()
    .find(|capability| capability.identifier == "workspace.capture-note")
    .expect("ready command capability");
  assert_eq!(ready.metadata["surface"], "command");
  assert_eq!(ready.metadata["definitionStatus"], "ready");

  let missing = registry
    .iter()
    .find(|capability| capability.identifier == "workspace.missing-note")
    .expect("missing command capability");
  assert_eq!(missing.metadata["surface"], "command");
  assert_eq!(missing.metadata["definitionStatus"], "missing");
  assert!(missing.metadata["definitionError"].contains("failed to read"));
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
    "kind": "builtin.workspaceReadmeNote",
    "input": {
      "envelope": "workspace.capture.input",
      "fields": [
        {
          "name": "workspace",
          "kind": "workspaceSummary",
          "required": true,
          "description": "Workspace to summarize."
        }
      ]
    },
    "output": {
      "envelope": "workspace.capture.output",
      "fields": [
        {
          "name": "note",
          "kind": "memoryNote",
          "required": false,
          "description": "Captured note."
        }
      ]
    }
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
  assert_eq!(execution.input.envelope, "workspace.capture.input");
  assert_eq!(execution.input.fields[0].name, "workspace");
  assert_eq!(execution.output.envelope, "workspace.capture.output");
  assert_eq!(execution.output.fields[0].kind, "memoryNote");
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
fn build_command_registry_applies_default_execution_contract() {
  let plugin_root = create_temp_plugin_root("command-default-contract");
  let plugin_dir = plugin_root.join("notion-runner");
  let commands_dir = plugin_dir.join("commands");
  fs::create_dir_all(&commands_dir).expect("create commands dir");
  fs::write(
    plugin_dir.join("pith-plugin.json"),
    r#"{
  "name": "notion-runner",
  "version": "0.1.0",
  "displayName": "Notion Runner",
  "description": "Test plugin",
  "author": { "name": "Pith" },
  "capabilities": [
    "command:notion.sync-page"
  ],
  "permissions": [
    "network.outbound"
  ],
  "defaultEnabled": true
}"#,
  )
  .expect("write plugin manifest");
  fs::write(
    commands_dir.join("notion.sync-page.json"),
    r#"{
  "title": "Sync Notion Page",
  "description": "Run the Notion page sync command.",
  "prompt": "Sync the selected Notion page.",
  "execution": {
    "kind": "stdio.notionSync",
    "entrypoint": "bin/notion-sync",
    "connectors": ["notion"]
  }
}"#,
  )
  .expect("write command definition");

  let plugins = discover_plugins(&plugin_root).expect("discover plugins");
  let commands = build_command_registry(&plugins);

  fs::remove_dir_all(&plugin_root).expect("cleanup plugin root");

  let execution = commands[0]
    .execution
    .as_ref()
    .expect("command execution contract");
  assert_eq!(execution.kind, "stdio.notionSync");
  assert_eq!(execution.driver, "stdio");
  assert_eq!(execution.entrypoint.as_deref(), Some("bin/notion-sync"));
  let connector_ids = execution.connector_ids.as_ref().expect("connector ids");
  assert_eq!(connector_ids.len(), 1);
  assert_eq!(connector_ids[0], "notion");
  assert_eq!(execution.input.envelope, "pith.plugin.command.input");
  assert_eq!(execution.input.fields.len(), 3);
  assert_eq!(execution.input.fields[0].name, "threadId");
  assert_eq!(execution.input.fields[1].name, "input");
  assert_eq!(execution.input.fields[2].name, "workspace");
  assert_eq!(execution.output.envelope, "pith.plugin.command.output");
  assert_eq!(execution.output.fields.len(), 4);
  assert_eq!(execution.output.fields[0].name, "content");
  assert_eq!(execution.output.fields[1].name, "message");
  assert_eq!(execution.output.fields[2].name, "items");
  assert!(!execution.output.fields[2].required);
  assert_eq!(execution.output.fields[3].name, "memoryNotes");
}

#[test]
fn build_command_registry_skips_unsafe_capability_identifiers() {
  let plugin_root = create_temp_plugin_root("command-unsafe-identifier");
  let plugin_dir = plugin_root.join("unsafe-command");
  let commands_dir = plugin_dir.join("commands");
  fs::create_dir_all(&commands_dir).expect("create commands dir");
  fs::write(
    plugin_dir.join("outside.json"),
    r#"{
  "title": "Outside Command",
  "description": "This command must not be loaded through a path escape.",
  "prompt": "Do not load this."
}"#,
  )
  .expect("write outside command definition");
  let plugin = PluginCatalogEntry {
    id: "unsafe-command".to_string(),
    name: "unsafe-command".to_string(),
    version: "0.1.0".to_string(),
    display_name: "Unsafe Command".to_string(),
    status: "ready".to_string(),
    description: "Manually constructed plugin".to_string(),
    author_name: Some("Pith".to_string()),
    enabled: true,
    default_enabled: true,
    capabilities: vec!["command:../outside".to_string()],
    permissions: vec!["file.read".to_string()],
    manifest_path: plugin_dir.join("pith-plugin.json").display().to_string(),
    provenance: "local".to_string(),
    validation_error: None,
    validation_hint: None,
  };

  let commands = build_command_registry(&[plugin]);

  fs::remove_dir_all(&plugin_root).expect("cleanup plugin root");

  assert!(commands.is_empty());
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
  assert_eq!(hooks[0].manifest_error, None);
  assert!(hooks[0].source_path.ends_with("shell.recorder.json"));
}

#[test]
fn build_hook_registry_surfaces_invalid_hook_manifests() {
  let plugin_root = create_temp_plugin_root("hook-invalid-manifest");
  let plugin_dir = plugin_root.join("broken-hook");
  let hooks_dir = plugin_dir.join("hooks");
  fs::create_dir_all(&hooks_dir).expect("create hooks dir");
  fs::write(hooks_dir.join("shell.broken.json"), "{").expect("write invalid hook definition");
  let plugin = PluginCatalogEntry {
    id: "broken-hook".to_string(),
    name: "broken-hook".to_string(),
    version: "0.1.0".to_string(),
    display_name: "Broken Hook".to_string(),
    status: "ready".to_string(),
    description: "Manually constructed plugin".to_string(),
    author_name: Some("Pith".to_string()),
    enabled: true,
    default_enabled: true,
    capabilities: vec!["hook:shell.broken".to_string()],
    permissions: vec!["shell.exec".to_string()],
    manifest_path: plugin_dir.join("pith-plugin.json").display().to_string(),
    provenance: "local".to_string(),
    validation_error: None,
    validation_hint: None,
  };

  let hooks = build_hook_registry(&[plugin]);

  fs::remove_dir_all(&plugin_root).expect("cleanup plugin root");

  assert_eq!(hooks.len(), 1);
  assert_eq!(hooks[0].hook_id, "broken-hook::shell.broken");
  assert_eq!(hooks[0].title, "shell.broken");
  assert_eq!(hooks[0].event, "invalid");
  assert!(hooks[0]
    .manifest_error
    .as_deref()
    .expect("manifest error")
    .contains("manifest could not be loaded"));
}

#[test]
fn build_hook_registry_skips_unsafe_capability_identifiers() {
  let plugin_root = create_temp_plugin_root("hook-unsafe-identifier");
  let plugin_dir = plugin_root.join("unsafe-hook");
  let hooks_dir = plugin_dir.join("hooks");
  fs::create_dir_all(&hooks_dir).expect("create hooks dir");
  fs::write(
    plugin_dir.join("outside.json"),
    r#"{
  "title": "Outside Hook",
  "description": "This hook must not be loaded through a path escape.",
  "event": "shell.completed",
  "messageTemplate": "Do not load this."
}"#,
  )
  .expect("write outside hook definition");
  let plugin = PluginCatalogEntry {
    id: "unsafe-hook".to_string(),
    name: "unsafe-hook".to_string(),
    version: "0.1.0".to_string(),
    display_name: "Unsafe Hook".to_string(),
    status: "ready".to_string(),
    description: "Manually constructed plugin".to_string(),
    author_name: Some("Pith".to_string()),
    enabled: true,
    default_enabled: true,
    capabilities: vec!["hook:../outside".to_string()],
    permissions: vec!["shell.exec".to_string()],
    manifest_path: plugin_dir.join("pith-plugin.json").display().to_string(),
    provenance: "local".to_string(),
    validation_error: None,
    validation_hint: None,
  };

  let hooks = build_hook_registry(&[plugin]);

  fs::remove_dir_all(&plugin_root).expect("cleanup plugin root");

  assert!(hooks.is_empty());
}
