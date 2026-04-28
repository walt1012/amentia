use std::path::Path;

use pith_protocol::{
  JsonRpcRequest, JsonRpcResponse, WorkspaceSearchMatch, WorkspaceSearchParams,
  WorkspaceSearchResult, WorkspaceSummary,
};
use pith_tools::search_files;

use crate::request_params::parse_required_params;
use crate::RuntimeContext;

#[derive(Debug)]
pub struct PreparedWorkspaceSearch {
  request_id: serde_json::Value,
  workspace: WorkspaceSummary,
  query: String,
  max_results: usize,
}

#[derive(Debug)]
pub struct CompletedWorkspaceSearch {
  request_id: serde_json::Value,
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
  complete_prepared_workspace_search(completed)
}

pub fn prepare_workspace_search(
  context: &mut RuntimeContext,
  request: JsonRpcRequest,
) -> std::result::Result<PreparedWorkspaceSearch, JsonRpcResponse> {
  let params = parse_required_params::<WorkspaceSearchParams>(&request, "workspace/search")?;

  let Some(workspace) = context.workspace.clone() else {
    return Err(JsonRpcResponse::error(
      request.id,
      -32040,
      "Open a workspace before searching",
    ));
  };

  Ok(PreparedWorkspaceSearch {
    request_id: request.id,
    workspace,
    query: params.query,
    max_results: params.max_results.unwrap_or(24).clamp(1, 100),
  })
}

pub fn execute_prepared_workspace_search(
  prepared: PreparedWorkspaceSearch,
) -> CompletedWorkspaceSearch {
  let PreparedWorkspaceSearch {
    request_id,
    workspace,
    query,
    max_results,
  } = prepared;

  let output = search_files(Path::new(&workspace.root_path), &query, max_results)
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

  CompletedWorkspaceSearch { request_id, output }
}

pub fn complete_prepared_workspace_search(completed: CompletedWorkspaceSearch) -> JsonRpcResponse {
  match completed.output {
    Ok(result) => JsonRpcResponse::success(completed.request_id, &result),
    Err((code, message)) => JsonRpcResponse::error(completed.request_id, code, message),
  }
}
