use std::fs;
use std::path::PathBuf;

use pith_memory::MemoryEvent;
use pith_protocol::{
  JsonRpcRequest, JsonRpcResponse, WorkspaceCurrentResult, WorkspaceOpenParams,
  WorkspaceOpenResult, WorkspaceSummary,
};

use crate::request_params::parse_required_params;
use crate::RuntimeContext;

pub(crate) fn handle_workspace_current(
  context: &RuntimeContext,
  request: JsonRpcRequest,
) -> JsonRpcResponse {
  JsonRpcResponse::success(
    request.id,
    &WorkspaceCurrentResult {
      workspace: context.workspace_state.current.clone(),
    },
  )
}

pub(crate) fn handle_workspace_open(
  context: &mut RuntimeContext,
  request: JsonRpcRequest,
) -> JsonRpcResponse {
  let params = match parse_required_params::<WorkspaceOpenParams>(&request, "workspace/open") {
    Ok(params) => params,
    Err(response) => return response,
  };

  let workspace_path = PathBuf::from(params.path);
  if !workspace_path.is_dir() {
    return JsonRpcResponse::error(request.id, -32020, "Workspace path is not a directory");
  }

  let resolved_path = match fs::canonicalize(&workspace_path) {
    Ok(path) => path,
    Err(error) => {
      return JsonRpcResponse::error(
        request.id,
        -32021,
        format!("Failed to resolve workspace path: {error}"),
      )
    }
  };

  let workspace = WorkspaceSummary {
    root_path: resolved_path.display().to_string(),
    display_name: resolved_path
      .file_name()
      .map(|name| name.to_string_lossy().into_owned())
      .filter(|name| !name.is_empty())
      .unwrap_or_else(|| resolved_path.display().to_string()),
  };
  context.workspace_state.current = Some(workspace.clone());

  if let Err(error) = context.persist_workspace() {
    return JsonRpcResponse::error(request.id, -32010, error.to_string());
  }

  if let Err(error) = context.remember(MemoryEvent::WorkspaceOpened {
    display_name: workspace.display_name.clone(),
    root_path: workspace.root_path.clone(),
  }) {
    return JsonRpcResponse::error(request.id, -32011, error.to_string());
  }

  JsonRpcResponse::success(
    request.id,
    &WorkspaceOpenResult {
      workspace,
      thread_count: context.thread_state.len(),
    },
  )
}
