use pith_protocol::{
  JsonRpcRequest, JsonRpcResponse, ThreadChangePreviewParams, ThreadChangePreviewResult,
  ThreadRevertChangesParams, ThreadRevertChangesResult, ThreadWorkspaceChangeSummary,
  TimelineItem,
};
use pith_storage::StoredWorkspaceChangeRecord;
use pith_tools::{revert_file_change, validate_file_change_revert};

use crate::request_params::parse_required_params;
use crate::requests::thread_requests::thread_has_active_work;
use crate::RuntimeContext;

pub(crate) fn handle_thread_change_preview(
  context: &RuntimeContext,
  request: JsonRpcRequest,
) -> JsonRpcResponse {
  let params = match parse_required_params::<ThreadChangePreviewParams>(
    &request,
    "thread/changePreview",
  ) {
    Ok(params) => params,
    Err(response) => return response,
  };

  if context.thread_state.find(&params.thread_id).is_none() {
    return JsonRpcResponse::error(request.id, -32004, "Thread not found");
  }

  let changes = match active_workspace_changes_for_thread(context, &params.thread_id) {
    Ok(changes) => changes,
    Err(error) => return JsonRpcResponse::error(request.id, -32010, error.to_string()),
  };

  JsonRpcResponse::success(
    request.id,
    &ThreadChangePreviewResult {
      thread_id: params.thread_id,
      changes: changes
        .iter()
        .map(workspace_change_summary)
        .collect::<Vec<_>>(),
    },
  )
}

pub(crate) fn handle_thread_revert_changes(
  context: &mut RuntimeContext,
  request: JsonRpcRequest,
) -> JsonRpcResponse {
  let params = match parse_required_params::<ThreadRevertChangesParams>(
    &request,
    "thread/revertChanges",
  ) {
    Ok(params) => params,
    Err(response) => return response,
  };

  if thread_has_active_work(context, &params.thread_id) {
    return JsonRpcResponse::error(
      request.id,
      -32012,
      "Cannot revert session changes while local work is running.",
    );
  }
  if context.thread_state.find(&params.thread_id).is_none() {
    return JsonRpcResponse::error(request.id, -32004, "Thread not found");
  }

  let changes = match active_workspace_changes_for_thread(context, &params.thread_id) {
    Ok(changes) => changes,
    Err(error) => return JsonRpcResponse::error(request.id, -32010, error.to_string()),
  };
  if changes.is_empty() {
    return JsonRpcResponse::success(
      request.id,
      &ThreadRevertChangesResult {
        thread_id: params.thread_id,
        reverted_count: 0,
        items: vec![thread_revert_noop_item()],
      },
    );
  }

  for change in changes.iter().rev() {
    if let Err(error) = validate_file_change_revert(
      std::path::Path::new(&change.workspace_root_path),
      &change.relative_path,
      &change.next_content,
    ) {
      return JsonRpcResponse::error(request.id, -32013, error.to_string());
    }
  }

  for change in changes.iter().rev() {
    if let Err(error) = revert_file_change(
      std::path::Path::new(&change.workspace_root_path),
      &change.relative_path,
      &change.next_content,
      change.previous_content.as_deref(),
    ) {
      return JsonRpcResponse::error(request.id, -32013, error.to_string());
    }
    if let Err(error) = context.mark_workspace_change_reverted(&change.id) {
      return JsonRpcResponse::error(request.id, -32010, error.to_string());
    }
  }

  let item = thread_revert_completed_item(&changes);
  if let Some(thread) = context.thread_state.find_mut(&params.thread_id) {
    thread.append_items(vec![item.clone()]);
  }
  if let Err(error) = context.persist_runtime_state() {
    return JsonRpcResponse::error(request.id, -32010, error.to_string());
  }

  JsonRpcResponse::success(
    request.id,
    &ThreadRevertChangesResult {
      thread_id: params.thread_id,
      reverted_count: changes.len(),
      items: vec![item],
    },
  )
}

fn active_workspace_changes_for_thread(
  context: &RuntimeContext,
  thread_id: &str,
) -> anyhow::Result<Vec<StoredWorkspaceChangeRecord>> {
  let changes = context
    .workspace_changes_for_thread(thread_id)?
    .into_iter()
    .filter(|change| change.reverted_at.is_none())
    .collect::<Vec<_>>();

  Ok(changes)
}

fn workspace_change_summary(change: &StoredWorkspaceChangeRecord) -> ThreadWorkspaceChangeSummary {
  ThreadWorkspaceChangeSummary {
    id: change.id.clone(),
    relative_path: change.relative_path.clone(),
    action: change.action.clone(),
    bytes_written: change.next_content.len(),
    will_delete_file: change.previous_content.is_none(),
  }
}

fn thread_revert_completed_item(changes: &[StoredWorkspaceChangeRecord]) -> TimelineItem {
  let changed_paths = changes
    .iter()
    .rev()
    .map(|change| format!("- {}", change.relative_path))
    .collect::<Vec<_>>()
    .join("\n");
  TimelineItem {
    kind: "toolResult".to_string(),
    title: "Session Changes Reverted".to_string(),
    content: format!(
      "Reverted {} approved workspace change(s):\n{}",
      changes.len(),
      changed_paths
    ),
    attributes: Some(std::collections::HashMap::from([
      ("action".to_string(), "thread.revertChanges".to_string()),
      ("revertedCount".to_string(), changes.len().to_string()),
    ])),
  }
}

fn thread_revert_noop_item() -> TimelineItem {
  TimelineItem {
    kind: "toolResult".to_string(),
    title: "No Session Changes To Revert".to_string(),
    content: "This session has no unreverted workspace changes.".to_string(),
    attributes: Some(std::collections::HashMap::from([
      ("action".to_string(), "thread.revertChanges".to_string()),
      ("revertedCount".to_string(), "0".to_string()),
    ])),
  }
}
