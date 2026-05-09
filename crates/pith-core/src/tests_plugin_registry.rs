use super::protocol_adapters::build_protocol_capability_registry;
use super::test_support::{
  bundled_manifest_plugin_entry, bundled_plugin_entry, replace_plugin_catalog, request,
};
use super::*;
use pith_plugin_host::PluginCatalogEntry;
use pith_protocol::methods;

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
  assert_eq!(connectors[0]["authType"], "oauth2");
  assert_eq!(connectors[0]["credentialStore"], "keychain");
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
