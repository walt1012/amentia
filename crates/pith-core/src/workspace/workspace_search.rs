use std::path::Path;

use pith_model_runtime::GenerationCancellation;
use pith_protocol::{
  JsonRpcRequest, JsonRpcResponse, WorkspaceSearchCancelRunningResult, WorkspaceSearchMatch,
  WorkspaceSearchParams, WorkspaceSearchResult, WorkspaceSummary,
};
use pith_tools::search_files_with_cancellation;

use crate::request_params::parse_required_params;
use crate::RuntimeContext;

#[derive(Debug)]
pub struct PreparedWorkspaceSearch {
  request_id: serde_json::Value,
  running_id: String,
  workspace: WorkspaceSummary,
  query: String,
  max_results: usize,
  cancellation: GenerationCancellation,
}

#[derive(Debug)]
pub struct CompletedWorkspaceSearch {
  request_id: serde_json::Value,
  running_id: String,
  output: std::result::Result<WorkspaceSearchResult, (i32, String)>,
}

pub(crate) fn handle_workspace_search(
  context: &mut RuntimeContext,
  request: JsonRpcRequest,
) -> JsonRpcResponse {
  let prepared = match prepare_workspace_search(context, request) {
    Ok(prepared) => prepared,
    Err(response) => return response,
  };
  let completed = execute_prepared_workspace_search(prepared);
  complete_prepared_workspace_search(context, completed)
}

pub(crate) fn handle_workspace_search_cancel_running(
  context: &mut RuntimeContext,
  request: JsonRpcRequest,
) -> JsonRpcResponse {
  let cancelled_count = context.execution_state.cancel_running_workspace_searches();
  JsonRpcResponse::success(
    request.id,
    &WorkspaceSearchCancelRunningResult { cancelled_count },
  )
}

pub fn prepare_workspace_search(
  context: &mut RuntimeContext,
  request: JsonRpcRequest,
) -> std::result::Result<PreparedWorkspaceSearch, JsonRpcResponse> {
  let params = parse_required_params::<WorkspaceSearchParams>(&request, "workspace/search")?;

  let Some(workspace) = context.workspace_state.current_cloned() else {
    return Err(JsonRpcResponse::error(
      request.id,
      -32040,
      "Open a workspace before searching",
    ));
  };

  let running_id = workspace_search_running_id(&request.id);
  let cancellation = GenerationCancellation::new();
  context
    .execution_state
    .insert_running_workspace_search(running_id.clone(), cancellation.clone());

  Ok(PreparedWorkspaceSearch {
    request_id: request.id,
    running_id,
    workspace,
    query: params.query,
    max_results: params.max_results.unwrap_or(24).clamp(1, 100),
    cancellation,
  })
}

pub fn execute_prepared_workspace_search(
  prepared: PreparedWorkspaceSearch,
) -> CompletedWorkspaceSearch {
  let PreparedWorkspaceSearch {
    request_id,
    running_id,
    workspace,
    query,
    max_results,
    cancellation,
  } = prepared;

  let output =
    search_files_with_cancellation(Path::new(&workspace.root_path), &query, max_results, || {
      cancellation.is_cancelled()
    })
    .map(|matches| WorkspaceSearchResult {
      query,
      workspace,
      matches: matches
        .into_iter()
        .map(|entry| WorkspaceSearchMatch {
          relative_path: entry.relative_path,
          line_number: entry.line_number,
          line: entry.line,
        })
        .collect(),
      })
    .map_err(|error| (-32041, error.to_string()));

  CompletedWorkspaceSearch {
    request_id,
    running_id,
    output,
  }
}

pub fn complete_prepared_workspace_search(
  context: &mut RuntimeContext,
  completed: CompletedWorkspaceSearch,
) -> JsonRpcResponse {
  context
    .execution_state
    .remove_running_workspace_search(&completed.running_id);
  match completed.output {
    Ok(result) => JsonRpcResponse::success(completed.request_id, &result),
    Err((code, message)) => JsonRpcResponse::error(completed.request_id, code, message),
  }
}

fn workspace_search_running_id(request_id: &serde_json::Value) -> String {
  format!("workspace-search:{request_id}")
}
