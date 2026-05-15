#[cfg(unix)]
use super::test_support::{
  create_temp_plugin_bundle, create_temp_workspace, replace_plugin_catalog, request,
};
#[cfg(unix)]
use super::{handle_request, RuntimeContext};
#[cfg(unix)]
use pith_plugin_host::PluginCatalogEntry;
#[cfg(unix)]
use pith_protocol::methods;
#[cfg(unix)]
use serde_json::{json, Value};
#[cfg(unix)]
use std::fs;

#[cfg(unix)]
#[test]
fn plugin_command_run_rejects_unsupported_mcp_content_only_output() {
  let items = run_mcp_content_case(McpContentCase {
    label: "plugin-command-mcp-unsupported-content",
    plugin_id: "mcp-image",
    display_name: "MCP Image",
    execution_kind: "mcp.localImage",
    thread_title: "MCP Image Thread",
    response_line: concat!(
      r#"{"jsonrpc":"2.0","id":2,"result":{"content":["#,
      r#"{"type":"image","data":"abc123","mimeType":"image/png"}"#,
      r#"]}}"#,
    ),
  });

  assert_eq!(items[0]["kind"], "pluginCommand");
  assert_eq!(items[1]["kind"], "warning");
  assert_eq!(
    items[1]["attributes"]["pluginRunnerFailureKind"],
    "mcpProtocol"
  );
  assert_eq!(
    items[1]["attributes"]["mcpProtocolStatus"],
    "unsupportedContent"
  );
  assert_eq!(items[1]["attributes"]["mcpContentCount"], "1");
  assert_eq!(items[1]["attributes"]["mcpTextContentCount"], "0");
  assert_eq!(items[1]["attributes"]["mcpUsableTextContentCount"], "0");
  assert_eq!(items[1]["attributes"]["mcpUnsupportedContentCount"], "1");
  assert_eq!(
    items[1]["attributes"]["mcpUnsupportedContentTypes"],
    "image"
  );
  assert!(items[1]["attributes"]["pluginRunnerRecoveryHint"]
    .as_str()
    .expect("recovery hint")
    .contains("structuredContent"));
  assert!(items[1]["content"]
    .as_str()
    .expect("warning content")
    .contains("unsupported non-text content"));
}

#[cfg(unix)]
#[test]
fn plugin_command_run_rejects_empty_mcp_text_content() {
  let items = run_mcp_content_case(McpContentCase {
    label: "plugin-command-mcp-empty-text",
    plugin_id: "mcp-empty",
    display_name: "MCP Empty",
    execution_kind: "mcp.localEmpty",
    thread_title: "MCP Empty Thread",
    response_line: concat!(
      r#"{"jsonrpc":"2.0","id":2,"result":{"content":["#,
      r#"{"type":"text","text":"   "}"#,
      r#"]}}"#,
    ),
  });

  assert_eq!(items[0]["kind"], "pluginCommand");
  assert_eq!(items[1]["kind"], "warning");
  assert_eq!(
    items[1]["attributes"]["pluginRunnerFailureKind"],
    "mcpProtocol"
  );
  assert_eq!(items[1]["attributes"]["mcpProtocolStatus"], "emptyContent");
  assert_eq!(items[1]["attributes"]["mcpContentCount"], "1");
  assert_eq!(items[1]["attributes"]["mcpTextContentCount"], "1");
  assert_eq!(items[1]["attributes"]["mcpUsableTextContentCount"], "0");
  assert_eq!(items[1]["attributes"]["mcpUnsupportedContentCount"], "0");
  assert!(items[1]["attributes"]["pluginRunnerRecoveryHint"]
    .as_str()
    .expect("recovery hint")
    .contains("non-empty"));
  assert!(items[1]["content"]
    .as_str()
    .expect("warning content")
    .contains("empty text content"));
}

#[cfg(unix)]
struct McpContentCase<'a> {
  label: &'a str,
  plugin_id: &'a str,
  display_name: &'a str,
  execution_kind: &'a str,
  thread_title: &'a str,
  response_line: &'a str,
}

#[cfg(unix)]
fn run_mcp_content_case(case: McpContentCase<'_>) -> Vec<Value> {
  use std::os::unix::fs::PermissionsExt;

  let mut context = RuntimeContext::new_in_memory();
  let source_root = create_temp_plugin_bundle(case.label, case.plugin_id, case.display_name);
  let workspace = create_temp_workspace(&format!("{}-workspace", case.label));
  let plugin_manifest = source_root.join("pith-plugin.json");
  let command_name = format!("{}.capture", case.plugin_id);
  let command_id = format!("{}::{}", case.plugin_id, command_name);
  let server_path = source_root.join("mcp-server.sh");
  fs::write(
    &plugin_manifest,
    format!(
      r#"{{
  "name": "{plugin_id}",
  "version": "0.1.0",
  "displayName": "{display_name}",
  "description": "MCP content contract test plugin",
  "author": {{ "name": "Pith" }},
  "capabilities": ["command:{command_name}", "mcp_server:local"],
  "permissions": ["mcp.connect"],
  "mcpServers": [
    {{
      "id": "local",
      "command": "mcp-server.sh",
      "transport": "stdio"
    }}
  ],
  "defaultEnabled": true
}}"#,
      plugin_id = case.plugin_id,
      display_name = case.display_name,
      command_name = command_name
    ),
  )
  .expect("write mcp content plugin manifest");
  fs::write(
    source_root
      .join("commands")
      .join(format!("{command_name}.json")),
    format!(
      r#"{{
  "title": "Capture MCP Content",
  "description": "Return MCP content for contract validation.",
  "prompt": "Capture MCP content.",
  "execution": {{
    "kind": "{execution_kind}",
    "driver": "mcp",
    "entrypoint": "local.capture"
  }}
}}"#,
      execution_kind = case.execution_kind
    ),
  )
  .expect("write mcp content command manifest");
  fs::write(&server_path, mcp_server_script(case.response_line)).expect("write mcp content server");
  let mut permissions = fs::metadata(&server_path)
    .expect("mcp content server metadata")
    .permissions();
  permissions.set_mode(0o755);
  fs::set_permissions(&server_path, permissions).expect("set mcp content server permissions");
  replace_plugin_catalog(
    &mut context,
    vec![PluginCatalogEntry {
      id: case.plugin_id.to_string(),
      name: case.plugin_id.to_string(),
      version: "0.1.0".to_string(),
      display_name: case.display_name.to_string(),
      status: "ready".to_string(),
      description: "MCP content contract test plugin".to_string(),
      author_name: Some("Pith".to_string()),
      enabled: true,
      default_enabled: true,
      capabilities: vec![
        format!("command:{command_name}"),
        "mcp_server:local".to_string(),
      ],
      permissions: vec!["mcp.connect".to_string()],
      manifest_path: plugin_manifest.display().to_string(),
      provenance: "test".to_string(),
      validation_error: None,
      validation_hint: None,
    }],
  );

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
        "title": case.thread_title
      })),
    ),
  );

  let response = handle_request(
    &mut context,
    request(
      methods::PLUGIN_COMMAND_RUN,
      Some(json!({
        "threadId": "thread-1",
        "commandId": command_id
      })),
    ),
  );

  fs::remove_dir_all(&workspace).expect("cleanup temp workspace");
  fs::remove_dir_all(source_root.parent().expect("plugin root")).expect("cleanup plugin source");

  assert!(response.error.is_none());
  let result = response.result.expect("command run result");
  result["items"].as_array().expect("items").clone()
}

#[cfg(unix)]
fn mcp_server_script(response_line: &str) -> String {
  let mut script = [
    "#!/bin/sh".to_string(),
    "cat >/dev/null".to_string(),
    "printf '{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{}}\\n'".to_string(),
    format!("cat <<'JSON'\n{response_line}\nJSON"),
  ]
  .join("\n");
  script.push('\n');
  script
}
