use cavell_protocol::{
  ApprovalRequest, ApprovalRespondParams, InitializeParams, ThreadReadResult, ThreadSummary,
  TimelineItem, TurnStartResult, WorkspaceOpenParams, WorkspaceOpenResult, WorkspaceSummary,
};
use std::collections::HashMap;

#[test]
fn initialize_params_uses_camel_case_fields() {
  let params = InitializeParams {
    client_info: cavell_protocol::ClientInfo {
      name: "cavell-tests".to_string(),
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
  };

  let value = serde_json::to_value(result).expect("serialize thread read result");
  assert!(value.get("thread").is_some());
  assert!(value.get("items").is_some());
  assert!(value.get("pendingApprovals").is_some());
}

#[test]
fn workspace_payloads_use_camel_case_fields() {
  let params = WorkspaceOpenParams {
    path: "/tmp/cavell".to_string(),
  };
  let result = WorkspaceOpenResult {
    workspace: WorkspaceSummary {
      root_path: "/tmp/cavell".to_string(),
      display_name: "cavell".to_string(),
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
