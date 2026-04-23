use cavell_protocol::{
  InitializeParams, ThreadReadResult, ThreadSummary, TimelineItem, TurnStartResult,
};

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
      },
      TimelineItem {
        kind: "assistantMessage".to_string(),
        title: "Assistant".to_string(),
        content: "Hi".to_string(),
      },
    ],
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
    }],
  };

  let value = serde_json::to_value(result).expect("serialize thread read result");
  assert!(value.get("thread").is_some());
  assert!(value.get("items").is_some());
}
