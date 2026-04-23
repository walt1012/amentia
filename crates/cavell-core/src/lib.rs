use cavell_protocol::{
  methods, HealthPingResult, InitializeParams, InitializeResult, JsonRpcRequest, JsonRpcResponse,
  ServerCapabilities, ServerInfo, ThreadListResult, ThreadStartParams, ThreadStartResult,
  ThreadSummary, TimelineItem, TurnStartParams, TurnStartResult,
};

#[derive(Debug, Clone)]
struct StoredThread {
  summary: ThreadSummary,
  turn_count: usize,
}

#[derive(Debug, Clone)]
pub struct RuntimeContext {
  server_name: String,
  server_version: String,
  threads: Vec<StoredThread>,
  next_thread_number: usize,
}

impl RuntimeContext {
  pub fn new() -> Self {
    Self {
      server_name: "cavell-runtime".to_string(),
      server_version: env!("CARGO_PKG_VERSION").to_string(),
      threads: vec![],
      next_thread_number: 1,
    }
  }
}

impl Default for RuntimeContext {
  fn default() -> Self {
    Self::new()
  }
}

pub fn handle_request(context: &mut RuntimeContext, request: JsonRpcRequest) -> JsonRpcResponse {
  match request.method.as_str() {
    methods::INITIALIZE => handle_initialize(context, request),
    methods::HEALTH_PING => JsonRpcResponse::success(
      request.id,
      &HealthPingResult {
        status: "ok".to_string(),
      },
    ),
    methods::THREAD_START => handle_thread_start(context, request),
    methods::THREAD_LIST => JsonRpcResponse::success(
      request.id,
      &ThreadListResult {
        threads: context
          .threads
          .iter()
          .map(|thread| thread.summary.clone())
          .collect(),
      },
    ),
    methods::TURN_START => handle_turn_start(context, request),
    _ => JsonRpcResponse::error(request.id, -32601, "Method not found"),
  }
}

fn handle_initialize(context: &RuntimeContext, request: JsonRpcRequest) -> JsonRpcResponse {
  let params = match request.params {
    Some(value) => match serde_json::from_value::<InitializeParams>(value) {
      Ok(params) => params,
      Err(error) => {
        return JsonRpcResponse::error(
          request.id,
          -32602,
          format!("Invalid initialize params: {error}"),
        )
      }
    },
    None => {
      return JsonRpcResponse::error(request.id, -32602, "Missing initialize params");
    }
  };

  let _client = params.client_info;

  JsonRpcResponse::success(
    request.id,
    &InitializeResult {
      server_info: ServerInfo {
        name: context.server_name.clone(),
        version: context.server_version.clone(),
      },
      protocol_version: "0.1.0".to_string(),
      capabilities: ServerCapabilities {
        supports_threads: true,
        supports_tools: false,
        supports_plugins: false,
      },
    },
  )
}

fn handle_thread_start(context: &mut RuntimeContext, request: JsonRpcRequest) -> JsonRpcResponse {
  let params = match request.params {
    Some(value) => match serde_json::from_value::<ThreadStartParams>(value) {
      Ok(params) => params,
      Err(error) => {
        return JsonRpcResponse::error(
          request.id,
          -32602,
          format!("Invalid thread/start params: {error}"),
        )
      }
    },
    None => {
      return JsonRpcResponse::error(request.id, -32602, "Missing thread/start params");
    }
  };

  let thread = ThreadSummary {
    id: format!("thread-{}", context.next_thread_number),
    title: params.title,
    status: "ready".to_string(),
  };
  context.next_thread_number += 1;
  context.threads.push(StoredThread {
    summary: thread.clone(),
    turn_count: 0,
  });

  JsonRpcResponse::success(request.id, &ThreadStartResult { thread })
}

fn handle_turn_start(context: &mut RuntimeContext, request: JsonRpcRequest) -> JsonRpcResponse {
  let params = match request.params {
    Some(value) => match serde_json::from_value::<TurnStartParams>(value) {
      Ok(params) => params,
      Err(error) => {
        return JsonRpcResponse::error(
          request.id,
          -32602,
          format!("Invalid turn/start params: {error}"),
        )
      }
    },
    None => {
      return JsonRpcResponse::error(request.id, -32602, "Missing turn/start params");
    }
  };

  let Some(thread) = context
    .threads
    .iter_mut()
    .find(|thread| thread.summary.id == params.thread_id)
  else {
    return JsonRpcResponse::error(request.id, -32004, "Thread not found");
  };

  thread.turn_count += 1;
  thread.summary.status = format!("{} turn(s)", thread.turn_count);

  let assistant_message = format!(
    "Cavell received your message in {} and is ready for the next runtime step: {}",
    thread.summary.title, params.message
  );
  let plan_message = format!(
    "Prepare the next local agent step for {} and keep thread state ready for future tool execution.",
    thread.summary.title
  );

  JsonRpcResponse::success(
    request.id,
    &TurnStartResult {
      turn_id: format!("{}-turn-{}", thread.summary.id, thread.turn_count),
      thread_id: thread.summary.id.clone(),
      items: vec![
        TimelineItem {
          kind: "userMessage".to_string(),
          title: "User".to_string(),
          content: params.message,
        },
        TimelineItem {
          kind: "plan".to_string(),
          title: "Plan".to_string(),
          content: plan_message,
        },
        TimelineItem {
          kind: "assistantMessage".to_string(),
          title: "Assistant".to_string(),
          content: assistant_message,
        },
      ],
    },
  )
}

#[cfg(test)]
mod tests {
  use super::*;
  use serde_json::{json, Value};

  fn request(method: &str, params: Option<Value>) -> JsonRpcRequest {
    JsonRpcRequest {
      id: json!(1),
      method: method.to_string(),
      params,
    }
  }

  #[test]
  fn initialize_request_returns_capabilities() {
    let mut context = RuntimeContext::new();
    let response = handle_request(
      &mut context,
      request(
        methods::INITIALIZE,
        Some(json!({
          "clientInfo": {
            "name": "cavell-tests",
            "version": "0.1.0"
          }
        })),
      ),
    );

    assert!(response.error.is_none());
    let result = response.result.expect("initialize result");
    assert_eq!(result["protocolVersion"], "0.1.0");
    assert_eq!(result["capabilities"]["supportsThreads"], true);
  }

  #[test]
  fn health_ping_returns_ok() {
    let mut context = RuntimeContext::new();
    let response = handle_request(&mut context, request(methods::HEALTH_PING, None));

    assert!(response.error.is_none());
    let result = response.result.expect("health result");
    assert_eq!(result["status"], "ok");
  }

  #[test]
  fn unknown_method_returns_json_rpc_error() {
    let mut context = RuntimeContext::new();
    let response = handle_request(&mut context, request("unknown/method", None));

    assert!(response.result.is_none());
    let error = response.error.expect("error payload");
    assert_eq!(error.code, -32601);
  }

  #[test]
  fn thread_start_persists_thread_for_future_lists() {
    let mut context = RuntimeContext::new();

    let start_response = handle_request(
      &mut context,
      request(
        methods::THREAD_START,
        Some(json!({
          "title": "First Thread"
        })),
      ),
    );
    assert!(start_response.error.is_none());

    let list_response = handle_request(&mut context, request(methods::THREAD_LIST, None));
    let result = list_response.result.expect("thread list result");
    let threads = result["threads"].as_array().expect("thread array");

    assert_eq!(threads.len(), 1);
    assert_eq!(threads[0]["title"], "First Thread");
  }

  #[test]
  fn turn_start_returns_user_and_assistant_messages() {
    let mut context = RuntimeContext::new();

    let _ = handle_request(
      &mut context,
      request(
        methods::THREAD_START,
        Some(json!({
          "title": "Chat Thread"
        })),
      ),
    );

    let turn_response = handle_request(
      &mut context,
      request(
        methods::TURN_START,
        Some(json!({
          "threadId": "thread-1",
          "message": "Hello runtime"
        })),
      ),
    );

    assert!(turn_response.error.is_none());
    let result = turn_response.result.expect("turn result");
    let items = result["items"].as_array().expect("items");

    assert_eq!(items.len(), 3);
    assert_eq!(items[0]["kind"], "userMessage");
    assert_eq!(items[1]["kind"], "plan");
    assert_eq!(items[2]["kind"], "assistantMessage");
  }
}
