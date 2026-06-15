use super::protocol_adapters::build_protocol_capability_registry;
use super::test_support::{
  bundled_manifest_plugin_entry, bundled_plugin_entry, create_temp_plugin_bundle,
  create_temp_workspace, replace_plugin_catalog, request,
};
use super::*;
use crate::plugins::plugin_command_approval::PLUGIN_COMMAND_CONNECTOR_APPROVAL_REASON;
use pith_plugin_host::PluginCatalogEntry;
use pith_protocol::methods;
use pith_storage::RuntimeStore;
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
  assert!(commands[0]["runRepairHint"].is_null());
}

#[test]
fn plugin_command_registry_surfaces_invalid_command_manifests() {
  let mut context = RuntimeContext::new_in_memory();
  let source_root = create_temp_plugin_bundle(
    "plugin-command-invalid-manifest",
    "broken-tools",
    "Broken Tools",
  );
  let plugin_manifest = source_root.join("pith-plugin.json");
  fs::write(
    source_root.join("commands").join("broken-tools.run.json"),
    "{",
  )
  .expect("write invalid command manifest");
  replace_plugin_catalog(
    &mut context,
    vec![PluginCatalogEntry {
      id: "broken-tools".to_string(),
      name: "broken-tools".to_string(),
      version: "0.1.0".to_string(),
      display_name: "Broken Tools".to_string(),
      status: "ready".to_string(),
      description: "Broken command plugin".to_string(),
      author_name: Some("Pith".to_string()),
      enabled: true,
      default_enabled: true,
      capabilities: vec!["command:broken-tools.run".to_string()],
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
  assert_eq!(commands[0]["commandId"], "broken-tools::broken-tools.run");
  assert_eq!(commands[0]["title"], "broken-tools.run");
  assert_eq!(commands[0]["runStatus"], "invalidCommandManifest");
  assert!(commands[0]["execution"].is_null());
  assert!(commands[0]["runBlocker"]
    .as_str()
    .expect("run blocker")
    .contains("manifest could not be loaded"));
  assert!(commands[0]["runRepairHint"]
    .as_str()
    .expect("repair hint")
    .contains("Fix the command manifest"));
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
    "driver": "remote",
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
  assert_eq!(commands[0]["execution"]["driver"], "remote");
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
  assert!(commands[0]["runRepairHint"]
    .as_str()
    .expect("repair hint")
    .contains("supported driver"));
}

#[test]
fn plugin_command_registry_blocks_mcp_commands_with_missing_server() {
  let mut context = RuntimeContext::new_in_memory();
  let source_root = create_temp_plugin_bundle(
    "plugin-command-mcp-missing-server",
    "notion-tools",
    "Notion Tools",
  );
  let plugin_manifest = source_root.join("pith-plugin.json");
  fs::write(
    source_root.join("commands").join("notion-tools.run.json"),
    r#"{
  "title": "Create Notion Task",
  "description": "Create a Notion task through MCP.",
  "prompt": "Create a Notion task.",
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
  assert_eq!(commands[0]["execution"]["supported"], true);
  assert_eq!(commands[0]["runStatus"], "runnerSetup");
  assert!(commands[0]["runBlocker"]
    .as_str()
    .expect("run blocker")
    .contains("not declared"));
  assert!(commands[0]["runRepairHint"]
    .as_str()
    .expect("repair hint")
    .contains("Declare the referenced MCP server"));
}

#[test]
fn plugin_command_registry_marks_mcp_execution_contracts_supported() {
  let mut context = RuntimeContext::new_in_memory();
  let source_root =
    create_temp_plugin_bundle("plugin-command-mcp-status", "notion-tools", "Notion Tools");
  let plugin_manifest = source_root.join("pith-plugin.json");
  let server_path = source_root.join("mcp-server.sh");
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

#[cfg(unix)]
#[test]
fn plugin_command_registry_blocks_mcp_without_declared_mcp_permission() {
  use std::os::unix::fs::PermissionsExt;

  let mut context = RuntimeContext::new_in_memory();
  let source_root = create_temp_plugin_bundle(
    "plugin-command-mcp-permission-status",
    "mcp-permission",
    "MCP Permission",
  );
  let plugin_manifest = source_root.join("pith-plugin.json");
  let server_path = source_root.join("mcp-server.sh");
  fs::write(
    &plugin_manifest,
    r#"{
  "name": "mcp-permission",
  "version": "0.1.0",
  "displayName": "MCP Permission",
  "description": "MCP command plugin missing permission",
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
  .expect("write mcp plugin manifest");
  fs::write(
    source_root.join("commands").join("mcp-permission.run.json"),
    r#"{
  "title": "Run MCP Permission",
  "description": "Run a local MCP server from the plugin bundle.",
  "prompt": "Run the MCP command.",
  "execution": {
    "kind": "mcp.localRun",
    "driver": "mcp",
    "entrypoint": "local.run"
  }
}"#,
  )
  .expect("write mcp command manifest");
  fs::write(
    &server_path,
    r#"#!/bin/sh
printf '{"jsonrpc":"2.0","id":1,"result":{}}\n'
"#,
  )
  .expect("write mcp server");
  let mut permissions = fs::metadata(&server_path)
    .expect("mcp server metadata")
    .permissions();
  permissions.set_mode(0o755);
  fs::set_permissions(&server_path, permissions).expect("make mcp server executable");
  replace_plugin_catalog(
    &mut context,
    vec![PluginCatalogEntry {
      id: "mcp-permission".to_string(),
      name: "mcp-permission".to_string(),
      version: "0.1.0".to_string(),
      display_name: "MCP Permission".to_string(),
      status: "ready".to_string(),
      description: "MCP permission plugin".to_string(),
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
  assert_eq!(commands[0]["execution"]["supported"], true);
  assert_eq!(commands[0]["runStatus"], "missingPermission");
  assert!(commands[0]["runBlocker"]
    .as_str()
    .expect("run blocker")
    .contains("mcp.connect"));
}

#[cfg(unix)]
#[test]
fn plugin_command_registry_blocks_non_executable_mcp_runner() {
  let mut context = RuntimeContext::new_in_memory();
  let source_root =
    create_temp_plugin_bundle("plugin-command-mcp-runner-setup", "mcp-setup", "MCP Setup");
  let plugin_manifest = source_root.join("pith-plugin.json");
  let server_path = source_root.join("mcp-server.sh");
  fs::write(
    &plugin_manifest,
    r#"{
  "name": "mcp-setup",
  "version": "0.1.0",
  "displayName": "MCP Setup",
  "description": "MCP command plugin",
  "author": { "name": "Pith" },
  "capabilities": ["command:mcp-setup.run", "mcp_server:local"],
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
  .expect("write mcp plugin manifest");
  fs::write(
    source_root.join("commands").join("mcp-setup.run.json"),
    r#"{
  "title": "Run MCP Setup",
  "description": "Run a local MCP server from the plugin bundle.",
  "prompt": "Run the MCP command.",
  "execution": {
    "kind": "mcp.localRun",
    "driver": "mcp",
    "entrypoint": "local.run"
  }
}"#,
  )
  .expect("write mcp command manifest");
  fs::write(
    &server_path,
    r#"#!/bin/sh
printf '{"jsonrpc":"2.0","id":1,"result":{}}\n'
"#,
  )
  .expect("write mcp server");
  replace_plugin_catalog(
    &mut context,
    vec![PluginCatalogEntry {
      id: "mcp-setup".to_string(),
      name: "mcp-setup".to_string(),
      version: "0.1.0".to_string(),
      display_name: "MCP Setup".to_string(),
      status: "ready".to_string(),
      description: "MCP setup plugin".to_string(),
      author_name: Some("Pith".to_string()),
      enabled: true,
      default_enabled: true,
      capabilities: vec![
        "command:mcp-setup.run".to_string(),
        "mcp_server:local".to_string(),
      ],
      permissions: vec!["mcp.connect".to_string()],
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
  assert_eq!(commands[0]["execution"]["supported"], true);
  assert_eq!(commands[0]["runStatus"], "runnerSetup");
  assert!(commands[0]["runBlocker"]
    .as_str()
    .expect("run blocker")
    .contains("not executable"));
}

#[test]
fn plugin_command_registry_marks_stdio_execution_contracts_supported() {
  let mut context = RuntimeContext::new_in_memory();
  let source_root =
    create_temp_plugin_bundle("plugin-command-stdio-status", "stdio-tools", "Stdio Tools");
  let plugin_manifest = source_root.join("pith-plugin.json");
  let runner_path = source_root.join("runner.sh");
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
  fs::write(
    &runner_path,
    r#"#!/bin/sh
printf '{"content":"ok"}\n'
"#,
  )
  .expect("write runner");
  #[cfg(unix)]
  {
    use std::os::unix::fs::PermissionsExt;

    let mut permissions = fs::metadata(&runner_path)
      .expect("runner metadata")
      .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&runner_path, permissions).expect("make runner executable");
  }
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

#[cfg(unix)]
#[test]
fn plugin_command_registry_blocks_non_executable_stdio_runner() {
  let mut context = RuntimeContext::new_in_memory();
  let source_root = create_temp_plugin_bundle(
    "plugin-command-stdio-runner-setup",
    "stdio-setup",
    "Stdio Setup",
  );
  let plugin_manifest = source_root.join("pith-plugin.json");
  let runner_path = source_root.join("runner.sh");
  fs::write(
    source_root.join("commands").join("stdio-setup.run.json"),
    r#"{
  "title": "Run Stdio Setup",
  "description": "Run a local stdio tool from the plugin bundle.",
  "prompt": "Run the local stdio tool.",
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
printf '{"content":"ok"}\n'
"#,
  )
  .expect("write runner");
  replace_plugin_catalog(
    &mut context,
    vec![PluginCatalogEntry {
      id: "stdio-setup".to_string(),
      name: "stdio-setup".to_string(),
      version: "0.1.0".to_string(),
      display_name: "Stdio Setup".to_string(),
      status: "ready".to_string(),
      description: "Stdio setup plugin".to_string(),
      author_name: Some("Pith".to_string()),
      enabled: true,
      default_enabled: true,
      capabilities: vec!["command:stdio-setup.run".to_string()],
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
  assert_eq!(commands[0]["execution"]["supported"], true);
  assert_eq!(commands[0]["runStatus"], "runnerSetup");
  assert!(commands[0]["runBlocker"]
    .as_str()
    .expect("run blocker")
    .contains("not executable"));
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
  let runner_path = source_root.join("runner.sh");
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
    "credentialStore": "local"
  },
  "defaultEnabled": true
}"#,
  )
  .expect("write connector command plugin manifest");
  fs::write(
    &runner_path,
    r#"#!/bin/sh
printf '{"content":"ok"}\n'
"#,
  )
  .expect("write runner");
  #[cfg(unix)]
  {
    use std::os::unix::fs::PermissionsExt;

    let mut permissions = fs::metadata(&runner_path)
      .expect("runner metadata")
      .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&runner_path, permissions).expect("make runner executable");
  }
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
  assert!(command["runRepairHint"]
    .as_str()
    .expect("repair hint")
    .contains("Authorize the connector"));

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
  let blocked_error = blocked_response.error.expect("auth blocker error");
  assert_eq!(blocked_error.code, -32058);
  let blocked_data = blocked_error.data.expect("auth blocker error data");
  assert_eq!(blocked_data["runStatus"], "needsConnectorAuth");
  assert_eq!(blocked_data["connectorId"], "notion-tools::notion");
  assert_eq!(blocked_data["connectorIds"], "notion-tools::notion");
  assert_eq!(
    blocked_data["runRepairHint"],
    "Authorize the connection before running this action."
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
  assert!(ready_registry["commands"][0]["runRepairHint"].is_null());
  assert_eq!(ready_registry["commands"][0]["approvalRequired"], true);
  assert_eq!(
    ready_registry["commands"][0]["approvalReason"],
    PLUGIN_COMMAND_CONNECTOR_APPROVAL_REASON
  );
}

#[test]
fn connector_backed_plugin_commands_report_runner_setup_before_connector_auth() {
  let mut context = RuntimeContext::new_in_memory();
  let source_root = create_temp_plugin_bundle(
    "plugin-command-connector-setup-first",
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
      "service": "notion"
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
  .expect("write connector command plugin manifest");
  fs::write(
    source_root.join("commands").join("notion-tools.sync.json"),
    r#"{
  "title": "Sync Notion",
  "description": "Sync local context to Notion.",
  "prompt": "Prepare a Notion sync payload.",
  "execution": {
    "kind": "stdio.notionSync",
    "entrypoint": "missing-runner.sh"
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

  fs::remove_dir_all(source_root.parent().expect("plugin root")).expect("cleanup plugin source");

  assert!(registry_response.error.is_none());
  let registry = registry_response
    .result
    .expect("connector setup registry result");
  let command = &registry["commands"][0];
  assert_eq!(command["runStatus"], "runnerSetup");
  assert_eq!(command["requiredConnectorIds"][0], "notion-tools::notion");
  assert_eq!(command["approvalRequired"], false);
  assert!(command["runBlocker"]
    .as_str()
    .expect("run blocker")
    .contains("entrypoint could not be resolved"));
  assert!(command["runRepairHint"]
    .as_str()
    .expect("repair hint")
    .contains("Add the runner file"));
  let error = blocked_response.error.expect("runner setup blocker");
  assert_eq!(error.code, -32053);
  assert!(error.message.contains("entrypoint could not be resolved"));
  let data = error.data.expect("runner setup error data");
  assert_eq!(data["runStatus"], "runnerSetup");
  assert!(data["runRepairHint"]
    .as_str()
    .expect("error repair hint")
    .contains("Add the runner file"));
}

#[test]
fn connector_backed_plugin_commands_can_scope_connector_requirements() {
  let mut context = RuntimeContext::new_in_memory();
  let source_root = create_temp_plugin_bundle(
    "plugin-command-scoped-connectors",
    "notion-tools",
    "Notion Tools",
  );
  let plugin_manifest = source_root.join("pith-plugin.json");
  let runner_path = source_root.join("runner.sh");
  fs::write(
    &plugin_manifest,
    r#"{
  "name": "notion-tools",
  "version": "0.1.0",
  "displayName": "Notion Tools",
  "description": "Notion connector command plugin",
  "author": { "name": "Pith" },
  "capabilities": [
    "command:notion-tools.status",
    "command:notion-tools.sync",
    "connector:notion"
  ],
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
  .expect("write connector command plugin manifest");
  fs::write(
    &runner_path,
    r#"#!/bin/sh
printf '{"content":"ok"}\n'
"#,
  )
  .expect("write runner");
  #[cfg(unix)]
  {
    use std::os::unix::fs::PermissionsExt;

    let mut permissions = fs::metadata(&runner_path)
      .expect("runner metadata")
      .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&runner_path, permissions).expect("make runner executable");
  }
  fs::write(
    source_root
      .join("commands")
      .join("notion-tools.status.json"),
    r#"{
  "title": "Show Notion Status",
  "description": "Show local setup status without contacting Notion.",
  "prompt": "Show setup status.",
  "execution": {
    "kind": "stdio.status",
    "entrypoint": "runner.sh",
    "connectors": []
  }
}"#,
  )
  .expect("write local status command manifest");
  fs::write(
    source_root.join("commands").join("notion-tools.sync.json"),
    r#"{
  "title": "Sync Notion",
  "description": "Sync local context to Notion.",
  "prompt": "Prepare a Notion sync payload.",
  "execution": {
    "kind": "stdio.notionSync",
    "entrypoint": "runner.sh",
    "connectors": ["notion"]
  }
}"#,
  )
  .expect("write scoped connector command manifest");
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
        "command:notion-tools.status".to_string(),
        "command:notion-tools.sync".to_string(),
        "connector:notion".to_string(),
      ],
      permissions: vec!["network.outbound".to_string()],
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
  let status = commands
    .iter()
    .find(|command| command["commandId"] == "notion-tools::notion-tools.status")
    .expect("status command");
  let sync = commands
    .iter()
    .find(|command| command["commandId"] == "notion-tools::notion-tools.sync")
    .expect("sync command");

  assert_eq!(status["runStatus"], "ready");
  assert_eq!(status["requiredConnectorIds"].as_array().unwrap().len(), 0);
  assert_eq!(status["approvalRequired"], false);
  assert_eq!(sync["runStatus"], "needsConnectorAuth");
  assert_eq!(sync["requiredConnectorIds"][0], "notion-tools::notion");
}

#[test]
fn connector_backed_plugin_commands_report_missing_declared_connectors() {
  let mut context = RuntimeContext::new_in_memory();
  let source_root = create_temp_plugin_bundle(
    "plugin-command-missing-connector",
    "notion-tools",
    "Notion Tools",
  );
  let plugin_manifest = source_root.join("pith-plugin.json");
  fs::write(
    source_root.join("commands").join("notion-tools.sync.json"),
    r#"{
  "title": "Sync Notion",
  "description": "Sync local context to Notion.",
  "prompt": "Prepare a Notion sync payload.",
  "execution": {
    "kind": "stdio.notionSync",
    "entrypoint": "runner.sh",
    "connectors": ["missing"]
  }
}"#,
  )
  .expect("write missing connector command manifest");
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
      capabilities: vec!["command:notion-tools.sync".to_string()],
      permissions: vec!["network.outbound".to_string()],
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
  let command = &result["commands"][0];
  assert_eq!(command["runStatus"], "missingConnector");
  assert_eq!(command["requiredConnectorIds"][0], "notion-tools::missing");
  assert!(command["runBlocker"]
    .as_str()
    .expect("run blocker")
    .contains("not declared"));
}

#[test]
fn connector_backed_plugin_commands_do_not_require_auth_for_optional_connectors() {
  let mut context = RuntimeContext::new_in_memory();
  let source_root = create_temp_plugin_bundle(
    "plugin-command-optional-connector",
    "browser-tools",
    "Browser Tools",
  );
  let plugin_manifest = source_root.join("pith-plugin.json");
  let runner_path = source_root.join("runner.sh");
  fs::write(
    &plugin_manifest,
    r#"{
  "name": "browser-tools",
  "version": "0.1.0",
  "displayName": "Browser Tools",
  "description": "Connector command plugin without required auth",
  "author": { "name": "Pith" },
  "capabilities": ["command:browser-tools.search", "connector:web"],
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
  .expect("write optional connector plugin manifest");
  fs::write(
    &runner_path,
    r#"#!/bin/sh
printf '{"content":"ok"}\n'
"#,
  )
  .expect("write runner");
  #[cfg(unix)]
  {
    use std::os::unix::fs::PermissionsExt;

    let mut permissions = fs::metadata(&runner_path)
      .expect("runner metadata")
      .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&runner_path, permissions).expect("make runner executable");
  }
  fs::write(
    source_root
      .join("commands")
      .join("browser-tools.search.json"),
    r#"{
  "title": "Search Web",
  "description": "Search the web through an auth-free connector.",
  "prompt": "Search the web.",
  "execution": {
    "kind": "stdio.webSearch",
    "entrypoint": "runner.sh",
    "connectors": ["web"]
  }
}"#,
  )
  .expect("write optional connector command manifest");
  replace_plugin_catalog(
    &mut context,
    vec![PluginCatalogEntry {
      id: "browser-tools".to_string(),
      name: "browser-tools".to_string(),
      version: "0.1.0".to_string(),
      display_name: "Browser Tools".to_string(),
      status: "ready".to_string(),
      description: "Connector command plugin without required auth".to_string(),
      author_name: Some("Pith".to_string()),
      enabled: true,
      default_enabled: true,
      capabilities: vec![
        "command:browser-tools.search".to_string(),
        "connector:web".to_string(),
      ],
      permissions: vec!["network.outbound".to_string()],
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
  let command = &result["commands"][0];
  assert_eq!(command["runStatus"], "ready");
  assert_eq!(command["declaredConnectorIds"][0], "browser-tools::web");
  assert_eq!(command["requiredConnectorIds"].as_array().unwrap().len(), 0);
  assert_eq!(command["approvalRequired"], false);
}

#[test]
fn connector_backed_stdio_commands_require_network_permission() {
  let mut context = RuntimeContext::new_in_memory();
  let source_root = create_temp_plugin_bundle(
    "plugin-command-stdio-network-permission",
    "stdio-network",
    "Stdio Network",
  );
  let plugin_manifest = source_root.join("pith-plugin.json");
  let runner_path = source_root.join("runner.sh");
  fs::write(
    &plugin_manifest,
    r#"{
  "name": "stdio-network",
  "version": "0.1.0",
  "displayName": "Stdio Network",
  "description": "Connector-backed stdio command plugin missing network permission",
  "author": { "name": "Pith" },
  "capabilities": ["command:stdio-network.sync", "connector:notion"],
  "permissions": [],
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
  .expect("write connector command plugin manifest");
  fs::write(
    &runner_path,
    r#"#!/bin/sh
printf '{"content":"ok"}\n'
"#,
  )
  .expect("write runner");
  #[cfg(unix)]
  {
    use std::os::unix::fs::PermissionsExt;

    let mut permissions = fs::metadata(&runner_path)
      .expect("runner metadata")
      .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&runner_path, permissions).expect("make runner executable");
  }
  fs::write(
    source_root.join("commands").join("stdio-network.sync.json"),
    r#"{
  "title": "Sync Stdio Network",
  "description": "Sync through a connector-backed stdio command.",
  "prompt": "Sync through stdio.",
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
      id: "stdio-network".to_string(),
      name: "stdio-network".to_string(),
      version: "0.1.0".to_string(),
      display_name: "Stdio Network".to_string(),
      status: "ready".to_string(),
      description: "Connector-backed stdio command plugin missing network permission".to_string(),
      author_name: Some("Pith".to_string()),
      enabled: true,
      default_enabled: true,
      capabilities: vec![
        "command:stdio-network.sync".to_string(),
        "connector:notion".to_string(),
      ],
      permissions: vec![],
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
  assert_eq!(commands[0]["execution"]["supported"], true);
  assert_eq!(commands[0]["runStatus"], "missingPermission");
  assert!(commands[0]["runBlocker"]
    .as_str()
    .expect("run blocker")
    .contains("network.outbound"));
  assert!(commands[0]["runBlocker"]
    .as_str()
    .expect("run blocker")
    .contains("plugin action"));
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
  assert_eq!(hooks[0]["status"], "ready");
  assert!(hooks[0]["runRepairHint"].is_null());
}

#[test]
fn plugin_hook_registry_surfaces_invalid_hook_manifests() {
  let mut context = RuntimeContext::new_in_memory();
  let source_root = create_temp_plugin_bundle(
    "plugin-hook-invalid-manifest",
    "broken-hooks",
    "Broken Hooks",
  );
  let plugin_manifest = source_root.join("pith-plugin.json");
  fs::create_dir_all(source_root.join("hooks")).expect("create hooks dir");
  fs::write(
    &plugin_manifest,
    r#"{
  "name": "broken-hooks",
  "version": "0.1.0",
  "displayName": "Broken Hooks",
  "description": "Broken hook plugin",
  "author": { "name": "Pith" },
  "capabilities": ["hook:shell.broken"],
  "permissions": ["shell.exec"],
  "defaultEnabled": true
}"#,
  )
  .expect("write hook plugin manifest");
  fs::write(source_root.join("hooks").join("shell.broken.json"), "{")
    .expect("write invalid hook manifest");
  replace_plugin_catalog(
    &mut context,
    vec![PluginCatalogEntry {
      id: "broken-hooks".to_string(),
      name: "broken-hooks".to_string(),
      version: "0.1.0".to_string(),
      display_name: "Broken Hooks".to_string(),
      status: "ready".to_string(),
      description: "Broken hook plugin".to_string(),
      author_name: Some("Pith".to_string()),
      enabled: true,
      default_enabled: true,
      capabilities: vec!["hook:shell.broken".to_string()],
      permissions: vec!["shell.exec".to_string()],
      manifest_path: plugin_manifest.display().to_string(),
      provenance: "test".to_string(),
      validation_error: None,
      validation_hint: None,
    }],
  );

  let response = handle_request(&mut context, request(methods::PLUGIN_HOOK_REGISTRY, None));

  fs::remove_dir_all(source_root.parent().expect("plugin root")).expect("cleanup plugin source");

  assert!(response.error.is_none());
  let result = response.result.expect("hook registry result");
  let hooks = result["hooks"].as_array().expect("hooks");
  assert_eq!(hooks.len(), 1);
  assert_eq!(hooks[0]["hookId"], "broken-hooks::shell.broken");
  assert_eq!(hooks[0]["title"], "shell.broken");
  assert_eq!(hooks[0]["event"], "invalid");
  assert_eq!(hooks[0]["status"], "invalidHookManifest");
  assert!(hooks[0]["runBlocker"]
    .as_str()
    .expect("run blocker")
    .contains("manifest could not be loaded"));
  assert!(hooks[0]["runRepairHint"]
    .as_str()
    .expect("repair hint")
    .contains("Fix the hook manifest"));
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
  assert_eq!(connectors[0]["authType"], "api_key");
  assert_eq!(connectors[0]["credentialStore"], "local");
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
fn plugin_connector_authorize_returns_repair_metadata_when_disabled() {
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
    request(
      methods::PLUGIN_CONNECTOR_AUTHORIZE,
      Some(json!({
        "connectorId": "notion-connector::notion",
        "credentialSecret": "notion-local-token"
      })),
    ),
  );

  let error = response.error.expect("connector auth error");
  assert_eq!(error.code, -32056);
  let data = error.data.expect("connector auth error data");
  assert_eq!(data["connectorId"], "notion-connector::notion");
  assert_eq!(data["pluginId"], "notion-connector");
  assert_eq!(data["connectorStatus"], "disabled");
  assert!(data["connectorRepairHint"]
    .as_str()
    .expect("connector repair hint")
    .contains("Enable the connector plugin"));
}

#[test]
fn plugin_connector_authorize_rejects_missing_secret_for_api_key_connectors() {
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

  let response = handle_request(
    &mut context,
    request(
      methods::PLUGIN_CONNECTOR_AUTHORIZE,
      Some(json!({
        "connectorId": "notion-connector::notion",
        "credentialSecret": "   "
      })),
    ),
  );

  let error = response.error.expect("connector auth secret error");
  assert_eq!(error.code, -32058);
  let data = error.data.expect("connector auth secret error data");
  assert_eq!(data["connectorId"], "notion-connector::notion");
  assert_eq!(data["pluginId"], "notion-connector");
  assert_eq!(data["connectorStatus"], "missingCredentialSecret");
  assert!(data["connectorRepairHint"]
    .as_str()
    .expect("connector repair hint")
    .contains("token or API key"));
}

#[test]
fn plugin_connector_authorize_returns_repair_metadata_when_storage_fails() {
  let mut context = RuntimeContext::new_in_memory();
  let storage_root = create_temp_workspace("connector-auth-failing-storage");
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
    vec![bundled_manifest_plugin_entry(
      "notion-connector",
      "Notion Connector",
      true,
      false,
      &["mcp_server:notion", "connector:notion"],
      &["network.outbound", "mcp.connect"],
    )],
  );

  let response = handle_request(
    &mut context,
    request(
      methods::PLUGIN_CONNECTOR_AUTHORIZE,
      Some(json!({
        "connectorId": "notion-connector::notion",
        "credentialSecret": "notion-local-token"
      })),
    ),
  );

  fs::remove_dir_all(&storage_root).expect("cleanup storage root");

  let error = response.error.expect("connector auth storage error");
  assert_eq!(error.code, -32010);
  let data = error.data.expect("connector auth storage error data");
  assert_eq!(data["connectorId"], "notion-connector::notion");
  assert_eq!(data["pluginId"], "notion-connector");
  assert_eq!(data["connectorStatus"], "credentialStoreError");
  assert!(data["connectorRepairHint"]
    .as_str()
    .expect("connector repair hint")
    .contains("storage permissions"));
}

#[test]
fn plugin_connector_clear_returns_repair_metadata_when_missing() {
  let mut context = RuntimeContext::new_in_memory();
  replace_plugin_catalog(&mut context, vec![]);

  let response = handle_request(
    &mut context,
    request(
      methods::PLUGIN_CONNECTOR_CLEAR_CREDENTIAL,
      Some(json!({
        "connectorId": "missing-connector::notion"
      })),
    ),
  );

  let error = response.error.expect("connector clear error");
  assert_eq!(error.code, -32055);
  let data = error.data.expect("connector clear error data");
  assert_eq!(data["connectorId"], "missing-connector::notion");
  assert_eq!(data["connectorStatus"], "missingConnector");
  assert!(data["connectorRepairHint"]
    .as_str()
    .expect("connector repair hint")
    .contains("Refresh plugins"));
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
