use pith_protocol::{
  ApprovalRequest, ApprovalRespondParams, InitializeParams, PluginCapabilityRegistration,
  PluginCapabilityRegistryResult, PluginCapabilityRegistrySummary, PluginCommandRegistryResult,
  PluginCommandRunParams, PluginCommandSummary, PluginSetEnabledParams, ThreadReadResult,
  ThreadSummary, TimelineItem, TurnStartResult, WorkspaceOpenParams, WorkspaceOpenResult,
  WorkspaceSummary,
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
      manifest_path: "plugins/official/review-assistant/pith-plugin.json".to_string(),
    }],
  };

  let encoded = serde_json::to_string(&result).expect("serialize capability registry");
  let decoded: PluginCapabilityRegistryResult =
    serde_json::from_str(&encoded).expect("deserialize capability registry");

  assert_eq!(decoded.summary.enabled_plugin_count, 2);
  assert_eq!(decoded.summary.total_capability_count, 3);
  assert_eq!(decoded.capabilities[0].plugin_id, "review-assistant");
  assert_eq!(decoded.capabilities[0].kind, "tool");
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
fn plugin_command_registry_round_trips() {
  let result = PluginCommandRegistryResult {
    commands: vec![PluginCommandSummary {
      command_id: "workspace-notes::workspace.capture-note".to_string(),
      title: "Capture Workspace Note".to_string(),
      description: "Prepare a reusable workspace note from the current context.".to_string(),
      plugin_id: "workspace-notes".to_string(),
      plugin_display_name: "Workspace Notes".to_string(),
      permissions: vec!["file.read".to_string(), "file.write".to_string()],
      source_path: "plugins/official/workspace-notes/commands/workspace.capture-note.json"
        .to_string(),
    }],
  };

  let encoded = serde_json::to_string(&result).expect("serialize command registry");
  let decoded: PluginCommandRegistryResult =
    serde_json::from_str(&encoded).expect("deserialize command registry");

  assert_eq!(decoded.commands.len(), 1);
  assert_eq!(decoded.commands[0].plugin_id, "workspace-notes");
  assert_eq!(decoded.commands[0].title, "Capture Workspace Note");
}
