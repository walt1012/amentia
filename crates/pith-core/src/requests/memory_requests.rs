use pith_protocol::{
  JsonRpcRequest, JsonRpcResponse, MemoryCreateParams, MemoryCreateResult, MemoryListResult,
};

use crate::protocol_adapters::{to_protocol_memory_note, to_protocol_memory_status};
use crate::request_params::parse_required_params;
use crate::RuntimeContext;

pub(crate) fn handle_memory_create(
  context: &mut RuntimeContext,
  request: JsonRpcRequest,
) -> JsonRpcResponse {
  let params = match parse_required_params::<MemoryCreateParams>(&request, "memory/create") {
    Ok(params) => params,
    Err(response) => return response,
  };

  let Some(workspace) = context.workspace_state.current.clone() else {
    return JsonRpcResponse::error(
      request.id,
      -32040,
      "Open a workspace before creating memory notes",
    );
  };

  let title = params.title.trim();
  let body = params.body.trim();
  if title.is_empty() || body.is_empty() {
    return JsonRpcResponse::error(
      request.id,
      -32602,
      "memory/create title and body must be non-empty",
    );
  }

  match context.create_memory_note(
    title.to_string(),
    body.to_string(),
    workspace.display_name,
    "user".to_string(),
    vec![
      "workspace".to_string(),
      "user".to_string(),
      "manual".to_string(),
    ],
  ) {
    Ok(note) => JsonRpcResponse::success(
      request.id,
      &MemoryCreateResult {
        note: to_protocol_memory_note(note),
      },
    ),
    Err(error) => JsonRpcResponse::error(request.id, -32041, error.to_string()),
  }
}

pub(crate) fn handle_memory_list(
  context: &RuntimeContext,
  request: JsonRpcRequest,
) -> JsonRpcResponse {
  JsonRpcResponse::success(
    request.id,
    &MemoryListResult {
      notes: context
        .memory_notes
        .iter()
        .take(16)
        .cloned()
        .map(to_protocol_memory_note)
        .collect(),
    },
  )
}

pub(crate) fn handle_memory_status(
  context: &RuntimeContext,
  request: JsonRpcRequest,
) -> JsonRpcResponse {
  JsonRpcResponse::success(
    request.id,
    &to_protocol_memory_status(context.memory_manager.status(&context.memory_notes)),
  )
}
