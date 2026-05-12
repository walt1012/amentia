use pith_protocol::{
  ApprovalRequest, ApprovalRespondParams, InitializeParams, PluginCapabilityRegistration,
  PluginCapabilityRegistryResult, PluginCapabilityRegistrySummary,
  PluginCommandEnvelopeFieldSummary, PluginCommandEnvelopeSummary,
  PluginCommandExecutionSummary, PluginCommandRegistryResult, PluginCommandRunParams,
  PluginCommandSummary, PluginConnectorRegistryResult, PluginConnectorSummary,
  PluginHookRegistryResult, PluginHookSummary, PluginInstallParams, PluginRemoveParams,
  PluginRemoveResult, PluginSetEnabledParams, PluginSummary, ThreadReadResult, ThreadSummary,
  TimelineItem, TurnStartResult, WorkspaceOpenParams, WorkspaceOpenResult, WorkspaceSummary,
};
use std::collections::HashMap;

#[test]
fn initialize_params_uses_camel_case_fields() {
  let params = InitializeParams {
    client_info: pith_protocol::ClientInfo {
      name: "pith-tests".to_string(),
      version: "0.1.0".to_string(),
    },
  };

  let value = serde_json::to_value(params).expect("serialize initialize params");
  assert!(value.get("clientInfo").is_some());
  assert!(value.get("client_info").is_none());
}

#[test]
fn turn_start_result_round_trips_timeline_items() {
  let result = TurnStartResult {
    turn_id: "thread-1-turn-1".to_string(),
    thread_id: "thread-1".to_string(),
    items: vec![
      TimelineItem {
        kind: "userMessage".to_string(),
        title: "User".to_string(),
        content: "Hello".to_string(),
        attributes: None,
      },
      TimelineItem {
        kind: "assistantMessage".to_string(),
        title: "Assistant".to_string(),
        content: "Hi".to_string(),
        attributes: Some(HashMap::from([(
          "source".to_string(),
          "runtime".to_string(),
        )])),
      },
    ],
    pending_approvals: vec![],
    active_turn_id: Some("thread-1-turn-1".to_string()),
  };

  let encoded = serde_json::to_string(&result).expect("serialize turn result");
  let decoded: TurnStartResult = serde_json::from_str(&encoded).expect("deserialize turn result");

  assert_eq!(decoded.thread_id, "thread-1");
  assert_eq!(decoded.items.len(), 2);
  assert_eq!(decoded.items[0].kind, "userMessage");
}

#[test]
fn thread_read_result_contains_items() {
  let result = ThreadReadResult {
    thread: ThreadSummary {
      id: "thread-1".to_string(),
      title: "Thread".to_string(),
      status: "ready".to_string(),
      workspace: None,
    },
    items: vec![TimelineItem {
      kind: "system".to_string(),
      title: "Thread Ready".to_string(),
      content: "Thread is ready.".to_string(),
      attributes: None,
    }],
    pending_approvals: vec![ApprovalRequest {
      id: "approval-1".to_string(),
      thread_id: "thread-1".to_string(),
      action: "write_file".to_string(),
      title: "Write README.md".to_string(),
      relative_path: "README.md".to_string(),
    }],
    active_turn_id: None,
  };

  let value = serde_json::to_value(result).expect("serialize thread read result");
  assert!(value.get("thread").is_some());
  assert!(value.get("items").is_some());
  assert!(value.get("pendingApprovals").is_some());
}

#[test]
fn workspace_payloads_use_camel_case_fields() {
  let params = WorkspaceOpenParams {
    path: "/tmp/pith".to_string(),
  };
  let result = WorkspaceOpenResult {
    workspace: WorkspaceSummary {
      root_path: "/tmp/pith".to_string(),
      display_name: "pith".to_string(),
    },
    thread_count: 2,
  };

  let params_value = serde_json::to_value(params).expect("serialize workspace params");
  let result_value = serde_json::to_value(result).expect("serialize workspace result");

  assert!(params_value.get("path").is_some());
  assert!(result_value.get("threadCount").is_some());
  assert!(result_value["workspace"].get("rootPath").is_some());
  assert!(result_value["workspace"].get("displayName").is_some());
}

#[test]
fn approval_respond_params_use_camel_case_fields() {
  let params = ApprovalRespondParams {
    approval_id: "approval-1".to_string(),
    decision: "approved".to_string(),
  };

  let value = serde_json::to_value(params).expect("serialize approval respond params");
  assert!(value.get("approvalId").is_some());
  assert!(value.get("decision").is_some());
}

#[test]
fn plugin_set_enabled_params_use_camel_case_fields() {
  let params = PluginSetEnabledParams {
    plugin_id: "workspace-notes".to_string(),
    enabled: true,
  };

  let value = serde_json::to_value(params).expect("serialize plugin set enabled params");
  assert!(value.get("pluginId").is_some());
  assert_eq!(
    value.get("enabled").and_then(|item| item.as_bool()),
    Some(true)
  );
}

#[test]
fn plugin_install_and_remove_payloads_use_camel_case_fields() {
  let install_params = PluginInstallParams {
    source_path: "/tmp/pith/plugins/focus-review".to_string(),
  };
  let remove_params = PluginRemoveParams {
    manifest_path: "/tmp/pith/plugins/local/focus-review/pith-plugin.json".to_string(),
  };
  let remove_result = PluginRemoveResult {
    plugin_id: "focus-review".to_string(),
    display_name: "Focus Review".to_string(),
    removed_path: "/tmp/pith/plugins/local/focus-review".to_string(),
  };

  let install_value =
    serde_json::to_value(install_params).expect("serialize plugin install params");
  let remove_value = serde_json::to_value(remove_params).expect("serialize plugin remove params");
  let result_value = serde_json::to_value(remove_result).expect("serialize plugin remove result");

  assert!(install_value.get("sourcePath").is_some());
  assert!(remove_value.get("manifestPath").is_some());
  assert!(result_value.get("pluginId").is_some());
  assert!(result_value.get("displayName").is_some());
  assert!(result_value.get("removedPath").is_some());
}

#[test]
fn plugin_summary_round_trips_validation_hint() {
  let plugin = PluginSummary {
    id: "broken-plugin".to_string(),
    name: "broken-plugin".to_string(),
    version: "invalid".to_string(),
    display_name: "Broken Plugin".to_string(),
    status: "invalid".to_string(),
    description: "Broken sample plugin".to_string(),
    author_name: None,
    enabled: false,
    default_enabled: false,
    capabilities: vec!["memory:sync".to_string()],
    permissions: vec![],
    manifest_path: "plugins/local/broken-plugin/pith-plugin.json".to_string(),
    provenance: "local".to_string(),
    validation_error: Some("plugin capability kind `memory` is not supported".to_string()),
    validation_hint: Some(
      "Use one of the supported capability kinds: command, agent, prompt_pack, hook, tool, mcp_server, skill, connector, settings.".to_string(),
    ),
  };

  let value = serde_json::to_value(&plugin).expect("serialize plugin summary");
  assert!(value.get("validationError").is_some());
  assert!(value.get("validationHint").is_some());

  let decoded: PluginSummary = serde_json::from_value(value).expect("deserialize plugin summary");
  assert_eq!(
    decoded.validation_hint.as_deref(),
    Some(
      "Use one of the supported capability kinds: command, agent, prompt_pack, hook, tool, mcp_server, skill, connector, settings."
    )
  );
}

#[test]
fn plugin_capability_registry_round_trips() {
  let result = PluginCapabilityRegistryResult {
    summary: PluginCapabilityRegistrySummary {
      enabled_plugin_count: 2,
      total_capability_count: 3,
      capability_counts_by_kind: HashMap::from([
        ("prompt_pack".to_string(), 1),
        ("tool".to_string(), 2),
      ]),
    },
    capabilities: vec![PluginCapabilityRegistration {
      capability_id: "review-assistant::tool:diff.summaries".to_string(),
      kind: "tool".to_string(),
      identifier: "diff.summaries".to_string(),
      plugin_id: "review-assistant".to_string(),
      plugin_display_name: "Review Assistant".to_string(),
      permissions: vec!["file.read".to_string(), "model.invoke".to_string()],
      manifest_path: "plugins/bundled/review-assistant/pith-plugin.json".to_string(),
      metadata: HashMap::from([("service".to_string(), "diff".to_string())]),
    }],
  };

  let encoded = serde_json::to_string(&result).expect("serialize capability registry");
  let decoded: PluginCapabilityRegistryResult =
    serde_json::from_str(&encoded).expect("deserialize capability registry");

  assert_eq!(decoded.summary.enabled_plugin_count, 2);
  assert_eq!(decoded.summary.total_capability_count, 3);
  assert_eq!(decoded.capabilities[0].plugin_id, "review-assistant");
  assert_eq!(decoded.capabilities[0].kind, "tool");
  assert_eq!(decoded.capabilities[0].metadata["service"], "diff");
}

#[test]
fn plugin_command_payloads_use_camel_case_fields() {
  let params = PluginCommandRunParams {
    thread_id: "thread-1".to_string(),
    command_id: "workspace-notes::workspace.capture-note".to_string(),
    input: Some("Focus on the README".to_string()),
  };

  let value = serde_json::to_value(params).expect("serialize plugin command params");
  assert!(value.get("threadId").is_some());
  assert!(value.get("commandId").is_some());
  assert!(value.get("input").is_some());
}

#[test]
fn plugin_connector_registry_round_trips() {
  let result = PluginConnectorRegistryResult {
    connectors: vec![PluginConnectorSummary {
      connector_id: "notion-connector::notion".to_string(),
      display_name: "Notion".to_string(),
      service: "notion".to_string(),
      plugin_id: "notion-connector".to_string(),
      plugin_display_name: "Notion Connector".to_string(),
      enabled: false,
      status: "disabled".to_string(),
      permissions: vec!["network.outbound".to_string(), "mcp.connect".to_string()],
      manifest_path: "plugins/bundled/notion-connector/pith-plugin.json".to_string(),
      homepage: Some("https://www.notion.so".to_string()),
      auth_type: Some("oauth2".to_string()),
      auth_required: true,
      auth_scopes: vec!["read_content".to_string(), "insert_content".to_string()],
      credential_store: Some("keychain".to_string()),
    }],
  };

  let encoded = serde_json::to_string(&result).expect("serialize connector registry");
  let decoded: PluginConnectorRegistryResult =
    serde_json::from_str(&encoded).expect("deserialize connector registry");
  let value = serde_json::to_value(&decoded).expect("serialize connector registry value");

  assert_eq!(decoded.connectors.len(), 1);
  assert_eq!(
    decoded.connectors[0].connector_id,
    "notion-connector::notion"
  );
  assert_eq!(decoded.connectors[0].status, "disabled");
  assert_eq!(decoded.connectors[0].auth_type.as_deref(), Some("oauth2"));
  assert!(value["connectors"][0].get("connectorId").is_some());
  assert!(value["connectors"][0].get("authRequired").is_some());
}

#[test]
fn plugin_command_registry_round_trips() {
  let result = PluginCommandRegistryResult {
    commands: vec![PluginCommandSummary {
      command_id: "workspace-notes::workspace.capture-note".to_string(),
      title: "Capture Workspace Note".to_string(),
      description: "Prepare a reusable workspace note from the current context.".to_string(),
      plugin_id: "workspace-notes".to_string(),
      plugin_display_name: "Workspace Notes".to_string(),
      permissions: vec!["file.read".to_string(), "file.write".to_string()],
      source_path: "plugins/bundled/workspace-notes/commands/workspace.capture-note.json"
        .to_string(),
      execution: Some(PluginCommandExecutionSummary {
        kind: "builtin.workspaceReadmeNote".to_string(),
        driver: "builtin".to_string(),
        entrypoint: None,
        input: PluginCommandEnvelopeSummary {
          envelope: "pith.plugin.command.input".to_string(),
          fields: vec![PluginCommandEnvelopeFieldSummary {
            name: "threadId".to_string(),
            kind: "string".to_string(),
            required: true,
            description: Some("Runtime thread identifier.".to_string()),
          }],
        },
        output: PluginCommandEnvelopeSummary {
          envelope: "pith.plugin.command.output".to_string(),
          fields: vec![PluginCommandEnvelopeFieldSummary {
            name: "items".to_string(),
            kind: "timelineItems".to_string(),
            required: true,
            description: Some("Timeline items to append.".to_string()),
          }],
        },
        supported: true,
      }),
      execution_kind: Some("builtin.workspaceReadmeNote".to_string()),
      memory_summary: Some("Stores a workspace memory note after execution.".to_string()),
    }],
  };

  let encoded = serde_json::to_string(&result).expect("serialize command registry");
  let decoded: PluginCommandRegistryResult =
    serde_json::from_str(&encoded).expect("deserialize command registry");

  assert_eq!(decoded.commands.len(), 1);
  assert_eq!(decoded.commands[0].plugin_id, "workspace-notes");
  assert_eq!(decoded.commands[0].title, "Capture Workspace Note");
  assert_eq!(
    decoded.commands[0].execution_kind.as_deref(),
    Some("builtin.workspaceReadmeNote")
  );
  assert_eq!(
    decoded.commands[0]
      .execution
      .as_ref()
      .map(|execution| execution.driver.as_str()),
    Some("builtin")
  );
  assert_eq!(
    decoded.commands[0]
      .execution
      .as_ref()
      .map(|execution| execution.input.envelope.as_str()),
    Some("pith.plugin.command.input")
  );
  assert_eq!(
    decoded.commands[0].memory_summary.as_deref(),
    Some("Stores a workspace memory note after execution.")
  );
}

#[test]
fn plugin_hook_registry_round_trips() {
  let result = PluginHookRegistryResult {
    hooks: vec![PluginHookSummary {
      hook_id: "shell-recorder::shell.recorder".to_string(),
      title: "Record Shell Completion".to_string(),
      description: "Capture a compact shell completion note in the thread timeline.".to_string(),
      event: "shell.completed".to_string(),
      plugin_id: "shell-recorder".to_string(),
      plugin_display_name: "Shell Recorder".to_string(),
      permissions: vec!["shell.exec".to_string()],
      source_path: "plugins/bundled/shell-recorder/hooks/shell.recorder.json".to_string(),
      memory_summary: Some("Stores shell completion memory after execution.".to_string()),
    }],
  };

  let encoded = serde_json::to_string(&result).expect("serialize hook registry");
  let decoded: PluginHookRegistryResult =
    serde_json::from_str(&encoded).expect("deserialize hook registry");

  assert_eq!(decoded.hooks.len(), 1);
  assert_eq!(decoded.hooks[0].plugin_id, "shell-recorder");
  assert_eq!(decoded.hooks[0].event, "shell.completed");
  assert_eq!(
    decoded.hooks[0].memory_summary.as_deref(),
    Some("Stores shell completion memory after execution.")
  );
}
