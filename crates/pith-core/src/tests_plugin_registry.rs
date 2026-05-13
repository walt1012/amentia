use super::protocol_adapters::build_protocol_capability_registry;
use super::test_support::{
  bundled_manifest_plugin_entry, bundled_plugin_entry, create_temp_plugin_bundle,
  replace_plugin_catalog, request,
};
use super::*;
use pith_plugin_host::PluginCatalogEntry;
use pith_protocol::methods;
use serde_json::json;
use std::fs;

#[test]
fn plugin_command_registry_lists_enabled_command_plugins() {
  let mut context = RuntimeContext::new_in_memory();
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

  let response = handle_request(
    &mut context,
    request(methods::PLUGIN_COMMAND_REGISTRY, None),
  );

  assert!(response.error.is_none());
  let result = response.result.expect("command registry result");
  let commands = result["commands"].as_array().expect("commands");
  assert_eq!(commands.len(), 1);
  assert_eq!(commands[0]["pluginId"], "workspace-notes");
  assert_eq!(commands[0]["title"], "Capture Workspace Note");
  assert_eq!(commands[0]["executionKind"], "builtin.workspaceReadmeNote");
  assert_eq!(
    commands[0]["execution"]["kind"],
    "builtin.workspaceReadmeNote"
  );
  assert_eq!(commands[0]["execution"]["driver"], "builtin");
  assert_eq!(
    commands[0]["execution"]["input"]["envelope"],
    "pith.plugin.command.input"
  );
  assert_eq!(
    commands[0]["execution"]["output"]["envelope"],
    "pith.plugin.command.output"
  );
  assert_eq!(commands[0]["execution"]["supported"], true);
  assert_eq!(commands[0]["runStatus"], "ready");
}

#[test]
fn plugin_command_registry_marks_unsupported_execution_contracts() {
  let mut context = RuntimeContext::new_in_memory();
  let source_root = create_temp_plugin_bundle(
    "plugin-command-contract-status",
    "notion-tools",
    "Notion Tools",
  );
  let plugin_manifest = source_root.join("pith-plugin.json");
  fs::write(
    source_root.join("commands").join("notion-tools.run.json"),
    r#"{
  "title": "Create Notion Task",
  "description": "Create a Notion task from the current thread.",
  "prompt": "Create a task in Notion from the current thread.",
  "execution": {
    "kind": "mcp.notionCreateTask",
    "driver": "mcp",
    "entrypoint": "notion.createTask",
    "input": {
      "envelope": "notion.createTask.input",
      "fields": [
        {
          "name": "title",
          "kind": "string",
          "required": true,
          "description": "Task title."
        }
      ]
    },
    "output": {
      "envelope": "notion.createTask.output",
      "fields": [
        {
          "name": "url",
          "kind": "url",
          "required": false,
          "description": "Created task URL."
        }
      ]
    }
  }
}"#,
  )
  .expect("write command manifest");
  replace_plugin_catalog(
    &mut context,
    vec![PluginCatalogEntry {
      id: "notion-tools".to_string(),
      name: "notion-tools".to_string(),
      version: "0.1.0".to_string(),
      display_name: "Notion Tools".to_string(),
      status: "ready".to_string(),
      description: "Notion command plugin".to_string(),
      author_name: Some("Pith".to_string()),
      enabled: true,
      default_enabled: true,
      capabilities: vec!["command:notion-tools.run".to_string()],
      permissions: vec!["network.outbound".to_string(), "mcp.connect".to_string()],
      manifest_path: plugin_manifest.display().to_string(),
      provenance: "test".to_string(),
      validation_error: None,
      validation_hint: None,
    }],
  );

  let response = handle_request(
    &mut context,
    request(methods::PLUGIN_COMMAND_REGISTRY, None),
  );

  fs::remove_dir_all(source_root.parent().expect("plugin root")).expect("cleanup plugin source");

  assert!(response.error.is_none());
  let result = response.result.expect("command registry result");
  let commands = result["commands"].as_array().expect("commands");
  assert_eq!(commands.len(), 1);
  assert_eq!(commands[0]["execution"]["driver"], "mcp");
  assert_eq!(commands[0]["execution"]["entrypoint"], "notion.createTask");
  assert_eq!(
    commands[0]["execution"]["input"]["envelope"],
    "notion.createTask.input"
  );
  assert_eq!(
    commands[0]["execution"]["input"]["fields"][0]["name"],
    "title"
  );
  assert_eq!(
    commands[0]["execution"]["output"]["envelope"],
    "notion.createTask.output"
  );
  assert_eq!(commands[0]["execution"]["supported"], false);
  assert_eq!(commands[0]["runStatus"], "unsupportedExecution");
}

#[test]
fn plugin_command_registry_marks_mcp_execution_contracts_supported() {
  let mut context = RuntimeContext::new_in_memory();
  let source_root =
    create_temp_plugin_bundle("plugin-command-mcp-status", "notion-tools", "Notion Tools");
  let plugin_manifest = source_root.join("pith-plugin.json");
  fs::write(
    &plugin_manifest,
    r#"{
  "name": "notion-tools",
  "version": "0.1.0",
  "displayName": "Notion Tools",
  "description": "Notion MCP command plugin",
  "author": { "name": "Pith" },
  "capabilities": ["command:notion-tools.create-task", "mcp_server:notion"],
  "permissions": ["network.outbound", "mcp.connect"],
  "mcpServers": [
    {
      "id": "notion",
      "command": "mcp-server.sh",
      "transport": "stdio"
    }
  ],
  "defaultEnabled": true
}"#,
  )
  .expect("write mcp command plugin manifest");
  fs::write(
    source_root
      .join("commands")
      .join("notion-tools.create-task.json"),
    r#"{
  "title": "Create Notion Task",
  "description": "Create a Notion task from the current thread.",
  "prompt": "Create a task in Notion from the current thread.",
  "execution": {
    "kind": "mcp.notionCreateTask",
    "driver": "mcp",
    "entrypoint": "notion.createTask"
  }
}"#,
  )
  .expect("write mcp command manifest");
  replace_plugin_catalog(
    &mut context,
    vec![PluginCatalogEntry {
      id: "notion-tools".to_string(),
      name: "notion-tools".to_string(),
      version: "0.1.0".to_string(),
      display_name: "Notion Tools".to_string(),
      status: "ready".to_string(),
      description: "Notion MCP command plugin".to_string(),
      author_name: Some("Pith".to_string()),
      enabled: true,
      default_enabled: true,
      capabilities: vec![
        "command:notion-tools.create-task".to_string(),
        "mcp_server:notion".to_string(),
      ],
      permissions: vec!["network.outbound".to_string(), "mcp.connect".to_string()],
      manifest_path: plugin_manifest.display().to_string(),
      provenance: "test".to_string(),
      validation_error: None,
      validation_hint: None,
    }],
  );

  let response = handle_request(
    &mut context,
    request(methods::PLUGIN_COMMAND_REGISTRY, None),
  );

  fs::remove_dir_all(source_root.parent().expect("plugin root")).expect("cleanup plugin source");

  assert!(response.error.is_none());
  let result = response.result.expect("command registry result");
  let commands = result["commands"].as_array().expect("commands");
  assert_eq!(commands.len(), 1);
  assert_eq!(commands[0]["execution"]["driver"], "mcp");
  assert_eq!(commands[0]["execution"]["entrypoint"], "notion.createTask");
  assert_eq!(commands[0]["execution"]["supported"], true);
  assert_eq!(commands[0]["runStatus"], "ready");
}

#[test]
fn plugin_command_registry_marks_stdio_execution_contracts_supported() {
  let mut context = RuntimeContext::new_in_memory();
  let source_root =
    create_temp_plugin_bundle("plugin-command-stdio-status", "stdio-tools", "Stdio Tools");
  let plugin_manifest = source_root.join("pith-plugin.json");
  fs::write(
    source_root.join("commands").join("stdio-tools.run.json"),
    r#"{
  "title": "Run Stdio Tool",
  "description": "Run a local stdio tool from the plugin bundle.",
  "prompt": "Run the local stdio tool.",
  "execution": {
    "kind": "stdio.echo",
    "entrypoint": "runner.sh"
  }
}"#,
  )
  .expect("write command manifest");
  replace_plugin_catalog(
    &mut context,
    vec![PluginCatalogEntry {
      id: "stdio-tools".to_string(),
      name: "stdio-tools".to_string(),
      version: "0.1.0".to_string(),
      display_name: "Stdio Tools".to_string(),
      status: "ready".to_string(),
      description: "Stdio command plugin".to_string(),
      author_name: Some("Pith".to_string()),
      enabled: true,
      default_enabled: true,
      capabilities: vec!["command:stdio-tools.run".to_string()],
      permissions: vec!["file.read".to_string()],
      manifest_path: plugin_manifest.display().to_string(),
      provenance: "test".to_string(),
      validation_error: None,
      validation_hint: None,
    }],
  );

  let response = handle_request(
    &mut context,
    request(methods::PLUGIN_COMMAND_REGISTRY, None),
  );

  fs::remove_dir_all(source_root.parent().expect("plugin root")).expect("cleanup plugin source");

  assert!(response.error.is_none());
  let result = response.result.expect("command registry result");
  let commands = result["commands"].as_array().expect("commands");
  assert_eq!(commands.len(), 1);
  assert_eq!(commands[0]["execution"]["driver"], "stdio");
  assert_eq!(commands[0]["execution"]["entrypoint"], "runner.sh");
  assert_eq!(commands[0]["execution"]["supported"], true);
  assert_eq!(commands[0]["runStatus"], "ready");
}

#[test]
fn connector_backed_plugin_commands_require_connector_auth() {
  let mut context = RuntimeContext::new_in_memory();
  let source_root = create_temp_plugin_bundle(
    "plugin-command-connector-auth",
    "notion-tools",
    "Notion Tools",
  );
  let plugin_manifest = source_root.join("pith-plugin.json");
  fs::write(
    &plugin_manifest,
    r#"{
  "name": "notion-tools",
  "version": "0.1.0",
  "displayName": "Notion Tools",
  "description": "Notion connector command plugin",
  "author": { "name": "Pith" },
  "capabilities": ["command:notion-tools.sync", "connector:notion"],
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
  .expect("write connector command plugin manifest");
  fs::write(
    source_root.join("commands").join("notion-tools.sync.json"),
    r#"{
  "title": "Sync Notion",
  "description": "Sync local context to Notion.",
  "prompt": "Prepare a Notion sync payload.",
  "execution": {
    "kind": "stdio.notionSync",
    "entrypoint": "runner.sh"
  }
}"#,
  )
  .expect("write connector command manifest");
  replace_plugin_catalog(
    &mut context,
    vec![PluginCatalogEntry {
      id: "notion-tools".to_string(),
      name: "notion-tools".to_string(),
      version: "0.1.0".to_string(),
      display_name: "Notion Tools".to_string(),
      status: "ready".to_string(),
      description: "Notion connector command plugin".to_string(),
      author_name: Some("Pith".to_string()),
      enabled: true,
      default_enabled: true,
      capabilities: vec![
        "command:notion-tools.sync".to_string(),
        "connector:notion".to_string(),
      ],
      permissions: vec!["network.outbound".to_string(), "mcp.connect".to_string()],
      manifest_path: plugin_manifest.display().to_string(),
      provenance: "test".to_string(),
      validation_error: None,
      validation_hint: None,
    }],
  );

  let registry_response = handle_request(
    &mut context,
    request(methods::PLUGIN_COMMAND_REGISTRY, None),
  );
  assert!(registry_response.error.is_none());
  let registry = registry_response
    .result
    .expect("connector command registry result");
  let command = &registry["commands"][0];
  assert_eq!(command["commandId"], "notion-tools::notion-tools.sync");
  assert_eq!(command["execution"]["supported"], true);
  assert_eq!(command["runStatus"], "needsConnectorAuth");
  assert_eq!(command["approvalRequired"], false);
  assert_eq!(command["requiredConnectorIds"][0], "notion-tools::notion");

  let blocked_response = handle_request(
    &mut context,
    request(
      methods::PLUGIN_COMMAND_RUN,
      Some(json!({
        "threadId": "thread-1",
        "commandId": "notion-tools::notion-tools.sync"
      })),
    ),
  );
  assert_eq!(
    blocked_response.error.expect("auth blocker error").code,
    -32058
  );

  let authorize_response = handle_request(
    &mut context,
    request(
      methods::PLUGIN_CONNECTOR_AUTHORIZE,
      Some(json!({
        "connectorId": "notion-tools::notion"
      })),
    ),
  );
  assert!(authorize_response.error.is_none());
  let ready_response = handle_request(
    &mut context,
    request(methods::PLUGIN_COMMAND_REGISTRY, None),
  );

  fs::remove_dir_all(source_root.parent().expect("plugin root")).expect("cleanup plugin source");

  let ready_registry = ready_response
    .result
    .expect("ready connector command registry result");
  assert_eq!(ready_registry["commands"][0]["runStatus"], "ready");
  assert_eq!(ready_registry["commands"][0]["approvalRequired"], true);
  assert_eq!(
    ready_registry["commands"][0]["approvalReason"],
    "Connector-backed plugin commands require approval before runner launch."
  );
}

#[test]
fn plugin_hook_registry_lists_enabled_hook_plugins() {
  let mut context = RuntimeContext::new_in_memory();
  replace_plugin_catalog(
    &mut context,
    vec![bundled_manifest_plugin_entry(
      "shell-recorder",
      "Shell Recorder",
      true,
      false,
      &["hook:shell.recorder", "tool:shell.timeline"],
      &["shell.exec"],
    )],
  );

  let response = handle_request(&mut context, request(methods::PLUGIN_HOOK_REGISTRY, None));

  assert!(response.error.is_none());
  let result = response.result.expect("hook registry result");
  let hooks = result["hooks"].as_array().expect("hooks");
  assert_eq!(hooks.len(), 1);
  assert_eq!(hooks[0]["pluginId"], "shell-recorder");
  assert_eq!(hooks[0]["event"], "shell.completed");
  assert_eq!(hooks[0]["title"], "Record Shell Completion");
}

#[test]
fn plugin_connector_registry_lists_disabled_connector_plugins() {
  let mut context = RuntimeContext::new_in_memory();
  replace_plugin_catalog(
    &mut context,
    vec![bundled_manifest_plugin_entry(
      "notion-connector",
      "Notion Connector",
      false,
      false,
      &["mcp_server:notion", "connector:notion"],
      &["network.outbound", "mcp.connect"],
    )],
  );

  let response = handle_request(
    &mut context,
    request(methods::PLUGIN_CONNECTOR_REGISTRY, None),
  );

  assert!(response.error.is_none());
  let result = response.result.expect("connector registry result");
  let connectors = result["connectors"].as_array().expect("connectors");
  assert_eq!(connectors.len(), 1);
  assert_eq!(connectors[0]["connectorId"], "notion-connector::notion");
  assert_eq!(connectors[0]["status"], "disabled");
  assert_eq!(connectors[0]["authStatus"], "disabled");
  assert_eq!(connectors[0]["credentialPresent"], false);
  assert_eq!(connectors[0]["credentialSecretPresent"], false);
  assert_eq!(connectors[0]["authType"], "oauth2");
  assert_eq!(connectors[0]["credentialStore"], "keychain");
}

#[test]
fn plugin_connector_auth_lifecycle_updates_connector_registry() {
  let mut context = RuntimeContext::new_in_memory();
  replace_plugin_catalog(
    &mut context,
    vec![bundled_manifest_plugin_entry(
      "notion-connector",
      "Notion Connector",
      true,
      false,
      &["mcp_server:notion", "connector:notion"],
      &["network.outbound", "mcp.connect"],
    )],
  );

  let initial_response = handle_request(
    &mut context,
    request(methods::PLUGIN_CONNECTOR_REGISTRY, None),
  );
  let initial_result = initial_response
    .result
    .expect("initial connector registry result");
  let initial_connector = &initial_result["connectors"][0];
  assert_eq!(initial_connector["status"], "needsAuth");
  assert_eq!(initial_connector["authStatus"], "needsAuth");
  assert_eq!(initial_connector["credentialPresent"], false);
  assert_eq!(initial_connector["credentialSecretPresent"], false);

  let authorize_response = handle_request(
    &mut context,
    request(
      methods::PLUGIN_CONNECTOR_AUTHORIZE,
      Some(json!({
        "connectorId": "notion-connector::notion",
        "credentialSecret": "notion-local-token"
      })),
    ),
  );
  assert!(authorize_response.error.is_none());
  let authorized_connector = authorize_response
    .result
    .expect("authorize connector result")["connector"]
    .clone();
  assert_eq!(authorized_connector["status"], "ready");
  assert_eq!(authorized_connector["authStatus"], "authorized");
  assert_eq!(authorized_connector["credentialPresent"], true);
  assert_eq!(authorized_connector["credentialSecretPresent"], true);
  assert_eq!(
    authorized_connector["credentialProvider"],
    "pith.localCredentialProvider"
  );
  assert_eq!(
    authorized_connector["credentialHandle"],
    "notion-connector::notion"
  );
  assert_eq!(
    authorized_connector["credentialLabel"],
    "Notion authorization marker"
  );
  assert!(authorized_connector["credentialUpdatedAt"].is_i64());

  let clear_response = handle_request(
    &mut context,
    request(
      methods::PLUGIN_CONNECTOR_CLEAR_CREDENTIAL,
      Some(json!({
        "connectorId": "notion-connector::notion"
      })),
    ),
  );
  assert!(clear_response.error.is_none());
  let cleared_connector = clear_response
    .result
    .expect("clear connector credential result")["connector"]
    .clone();
  assert_eq!(cleared_connector["status"], "needsAuth");
  assert_eq!(cleared_connector["authStatus"], "needsAuth");
  assert_eq!(cleared_connector["credentialPresent"], false);
  assert_eq!(cleared_connector["credentialSecretPresent"], false);
  assert!(cleared_connector["credentialProvider"].is_null());
  assert!(cleared_connector["credentialHandle"].is_null());
}

#[test]
fn capability_registry_only_includes_ready_enabled_plugins() {
  let plugins = vec![
    bundled_plugin_entry(
      "workspace-notes",
      "Workspace Notes",
      true,
      true,
      &["prompt_pack:workspace.notes", "settings:workspace.preferences"],
      &["file.read", "file.write"],
    ),
    bundled_plugin_entry(
      "shell-recorder",
      "Shell Recorder",
      false,
      false,
      &["hook:shell.recorder"],
      &["shell.exec"],
    ),
    PluginCatalogEntry {
      id: "broken-plugin".to_string(),
      name: "broken-plugin".to_string(),
      version: "0.1.0".to_string(),
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
        "Use one of the supported capability kinds: command, agent, prompt_pack, hook, tool, mcp_server, skill, connector, settings.".to_string(),
      ),
    },
  ];

  let result = build_protocol_capability_registry(&plugins);

  assert_eq!(result.summary.enabled_plugin_count, 1);
  assert_eq!(result.summary.total_capability_count, 2);
  assert_eq!(
    result.summary.capability_counts_by_kind.get("prompt_pack"),
    Some(&1)
  );
  assert_eq!(
    result.summary.capability_counts_by_kind.get("settings"),
    Some(&1)
  );
  assert_eq!(result.capabilities.len(), 2);
  assert_eq!(result.capabilities[0].kind, "prompt_pack");
  assert_eq!(result.capabilities[0].plugin_id, "workspace-notes");
  assert_eq!(result.capabilities[1].kind, "settings");
}
