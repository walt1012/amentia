use anyhow::Result;
use cavell_protocol::{
  methods, HealthPingResult, InitializeParams, InitializeResult, JsonRpcRequest, JsonRpcResponse,
  ServerCapabilities, ServerInfo, ThreadListResult, ThreadReadParams, ThreadReadResult,
  ThreadStartParams, ThreadStartResult, ThreadSummary, TimelineItem, TurnStartParams,
  TurnStartResult,
};
use cavell_storage::{FileThreadStore, StoredThreadRecord};

#[derive(Debug, Clone)]
struct StoredThread {
  summary: ThreadSummary,
  turn_count: usize,
  items: Vec<TimelineItem>,
}

#[derive(Debug, Clone)]
pub struct RuntimeContext {
  server_name: String,
  server_version: String,
  store: Option<FileThreadStore>,
  threads: Vec<StoredThread>,
  next_thread_number: usize,
}

impl RuntimeContext {
  pub fn new() -> Result<Self> {
    let store = FileThreadStore::new_default()?;
    let persisted_threads = store.load_threads()?;
    let next_thread_number = persisted_threads.len() + 1;

    Ok(Self {
      server_name: "cavell-runtime".to_string(),
      server_version: env!("CARGO_PKG_VERSION").to_string(),
      store: Some(store),
      threads: persisted_threads
        .into_iter()
        .map(|thread| StoredThread {
          summary: thread.summary,
          turn_count: thread.turn_count,
          items: thread.items,
        })
        .collect(),
      next_thread_number,
    })
  }

  pub fn new_in_memory() -> Self {
    Self {
      server_name: "cavell-runtime".to_string(),
      server_version: env!("CARGO_PKG_VERSION").to_string(),
      store: None,
      threads: vec![],
      next_thread_number: 1,
    }
  }

  fn persist_threads(&self) -> Result<()> {
    let Some(store) = &self.store else {
      return Ok(());
    };

    let threads = self
      .threads
      .iter()
      .map(|thread| StoredThreadRecord {
        summary: thread.summary.clone(),
        turn_count: thread.turn_count,
        items: thread.items.clone(),
      })
      .collect::<Vec<_>>();

    store.save_threads(&threads)
  }
}

impl Default for RuntimeContext {
  fn default() -> Self {
    Self::new_in_memory()
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
    methods::THREAD_READ => handle_thread_read(context, request),
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

fn handle_thread_read(context: &RuntimeContext, request: JsonRpcRequest) -> JsonRpcResponse {
  let params = match request.params {
    Some(value) => match serde_json::from_value::<ThreadReadParams>(value) {
      Ok(params) => params,
      Err(error) => {
        return JsonRpcResponse::error(
          request.id,
          -32602,
          format!("Invalid thread/read params: {error}"),
        )
      }
    },
    None => {
      return JsonRpcResponse::error(request.id, -32602, "Missing thread/read params");
    }
  };

  let Some(thread) = context
    .threads
    .iter()
    .find(|thread| thread.summary.id == params.thread_id)
  else {
    return JsonRpcResponse::error(request.id, -32004, "Thread not found");
  };

  JsonRpcResponse::success(
    request.id,
    &ThreadReadResult {
      thread: thread.summary.clone(),
      items: thread.items.clone(),
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
  let items = vec![TimelineItem {
    kind: "system".to_string(),
    title: "Thread Ready".to_string(),
    content: format!("{} is ready for local runtime messages.", thread.title),
  }];
  context.threads.push(StoredThread {
    summary: thread.clone(),
    turn_count: 0,
    items: items.clone(),
  });

  if let Err(error) = context.persist_threads() {
    return JsonRpcResponse::error(request.id, -32010, error.to_string());
  }

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
  let turn_count = thread.turn_count;
  let thread_id = thread.summary.id.clone();
  let thread_title = thread.summary.title.clone();

  let assistant_message = format!(
    "Cavell received your message in {} and is ready for the next runtime step: {}",
    thread_title, params.message
  );
  let plan_message = format!(
    "Prepare the next local agent step for {} and keep thread state ready for future tool execution.",
    thread_title
  );

  let items = vec![
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
  ];
  thread.items.extend(items.clone());

  if let Err(error) = context.persist_threads() {
    return JsonRpcResponse::error(request.id, -32010, error.to_string());
  }

  JsonRpcResponse::success(
    request.id,
    &TurnStartResult {
      turn_id: format!("{thread_id}-turn-{turn_count}"),
      thread_id,
      items,
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
    let mut context = RuntimeContext::new_in_memory();
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
    let mut context = RuntimeContext::new_in_memory();
    let response = handle_request(&mut context, request(methods::HEALTH_PING, None));

    assert!(response.error.is_none());
    let result = response.result.expect("health result");
    assert_eq!(result["status"], "ok");
  }

  #[test]
  fn unknown_method_returns_json_rpc_error() {
    let mut context = RuntimeContext::new_in_memory();
    let response = handle_request(&mut context, request("unknown/method", None));

    assert!(response.result.is_none());
    let error = response.error.expect("error payload");
    assert_eq!(error.code, -32601);
  }

  #[test]
  fn thread_start_persists_thread_for_future_lists() {
    let mut context = RuntimeContext::new_in_memory();

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
  fn thread_read_returns_persisted_thread_items() {
    let mut context = RuntimeContext::new_in_memory();

    let _ = handle_request(
      &mut context,
      request(
        methods::THREAD_START,
        Some(json!({
          "title": "Inspectable Thread"
        })),
      ),
    );

    let read_response = handle_request(
      &mut context,
      request(
        methods::THREAD_READ,
        Some(json!({
          "threadId": "thread-1"
        })),
      ),
    );

    assert!(read_response.error.is_none());
    let result = read_response.result.expect("thread read result");
    let items = result["items"].as_array().expect("thread items");

    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["kind"], "system");
  }

  #[test]
  fn turn_start_returns_user_and_assistant_messages() {
    let mut context = RuntimeContext::new_in_memory();

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
