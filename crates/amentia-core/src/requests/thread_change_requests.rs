use amentia_protocol::{
  JsonRpcRequest, JsonRpcResponse, ThreadChangePreviewParams, ThreadChangePreviewResult,
  ThreadRevertChangesParams, ThreadRevertChangesResult, ThreadWorkspaceChangeSummary, TimelineItem,
};
use amentia_storage::StoredWorkspaceChangeRecord;
use amentia_tools::{revert_file_change, validate_file_change_revert};

use crate::request_params::parse_required_params;
use crate::requests::thread_requests::thread_has_active_work;
use crate::RuntimeContext;

struct ThreadFileRevertPlan {
  workspace_root_path: String,
  relative_path: String,
  action: String,
  previous_content: Option<Vec<u8>>,
  expected_current_content: Vec<u8>,
  change_ids: Vec<String>,
}

pub(crate) fn handle_thread_change_preview(
  context: &RuntimeContext,
  request: JsonRpcRequest,
) -> JsonRpcResponse {
  let params =
    match parse_required_params::<ThreadChangePreviewParams>(&request, "thread/changePreview") {
      Ok(params) => params,
      Err(response) => return response,
    };

  if context.thread_state.find(&params.thread_id).is_none() {
    return JsonRpcResponse::error(request.id, -32004, "Session not found");
  }

  let changes = match active_workspace_changes_for_thread(context, &params.thread_id) {
    Ok(changes) => changes,
    Err(error) => return JsonRpcResponse::error(request.id, -32010, error.to_string()),
  };

  let revert_plan = revert_plan_for_changes(&changes);

  JsonRpcResponse::success(
    request.id,
    &ThreadChangePreviewResult {
      thread_id: params.thread_id,
      changes: revert_plan
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
  let params =
    match parse_required_params::<ThreadRevertChangesParams>(&request, "thread/revertChanges") {
      Ok(params) => params,
      Err(response) => return response,
    };

  if thread_has_active_work(context, &params.thread_id) {
    return JsonRpcResponse::error(
      request.id,
      -32012,
      "Cannot revert session changes while current work is running.",
    );
  }
  if context.thread_state.find(&params.thread_id).is_none() {
    return JsonRpcResponse::error(request.id, -32004, "Session not found");
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
  let revert_plan = revert_plan_for_changes(&changes);

  for plan in revert_plan.iter().rev() {
    if let Err(error) = validate_file_change_revert(
      std::path::Path::new(&plan.workspace_root_path),
      &plan.relative_path,
      &plan.expected_current_content,
    ) {
      return JsonRpcResponse::error(request.id, -32013, error.to_string());
    }
  }

  for plan in revert_plan.iter().rev() {
    if let Err(error) = revert_file_change(
      std::path::Path::new(&plan.workspace_root_path),
      &plan.relative_path,
      &plan.expected_current_content,
      plan.previous_content.as_deref(),
    ) {
      return JsonRpcResponse::error(request.id, -32013, error.to_string());
    }
    for change_id in &plan.change_ids {
      if let Err(error) = context.mark_workspace_change_reverted(change_id) {
        return JsonRpcResponse::error(request.id, -32010, error.to_string());
      }
    }
  }

  let item = thread_revert_completed_item(&revert_plan);
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
      reverted_count: revert_plan.len(),
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

fn revert_plan_for_changes(changes: &[StoredWorkspaceChangeRecord]) -> Vec<ThreadFileRevertPlan> {
  let mut plan = Vec::<ThreadFileRevertPlan>::new();

  for change in changes {
    if let Some(existing) = plan.iter_mut().find(|entry| {
      entry.workspace_root_path == change.workspace_root_path
        && entry.relative_path == change.relative_path
    }) {
      existing.action = change.action.clone();
      existing.expected_current_content = change.next_content.clone();
      existing.change_ids.push(change.id.clone());
      continue;
    }

    plan.push(ThreadFileRevertPlan {
      workspace_root_path: change.workspace_root_path.clone(),
      relative_path: change.relative_path.clone(),
      action: change.action.clone(),
      previous_content: change.previous_content.clone(),
      expected_current_content: change.next_content.clone(),
      change_ids: vec![change.id.clone()],
    });
  }

  plan
}

fn workspace_change_summary(change: &ThreadFileRevertPlan) -> ThreadWorkspaceChangeSummary {
  let conflict_reason = validate_file_change_revert(
    std::path::Path::new(&change.workspace_root_path),
    &change.relative_path,
    &change.expected_current_content,
  )
  .err()
  .map(|error| error.to_string());

  ThreadWorkspaceChangeSummary {
    id: change.change_ids.join("+"),
    relative_path: change.relative_path.clone(),
    action: change.action.clone(),
    bytes_written: change.expected_current_content.len(),
    will_delete_file: change.previous_content.is_none(),
    can_revert: conflict_reason.is_none(),
    conflict_reason,
  }
}

fn thread_revert_completed_item(changes: &[ThreadFileRevertPlan]) -> TimelineItem {
  let changed_paths = changes
    .iter()
    .rev()
    .map(|change| format!("- {}", change.relative_path))
    .collect::<Vec<_>>()
    .join("\n");
  TimelineItem {
    kind: "toolResult".to_string(),
    title: "Session Changes Reverted".to_string(),
    content: format!("{}:\n{}", revert_count_line(changes.len()), changed_paths),
    attributes: Some(std::collections::HashMap::from([
      ("action".to_string(), "thread.revertChanges".to_string()),
      ("receiptKind".to_string(), "sessionChangeRevert".to_string()),
      ("revertedCount".to_string(), changes.len().to_string()),
    ])),
  }
}

fn revert_count_line(reverted_count: usize) -> String {
  match reverted_count {
    1 => "Reverted 1 file saved by this session".to_string(),
    count => format!("Reverted {count} files saved by this session"),
  }
}

fn thread_revert_noop_item() -> TimelineItem {
  TimelineItem {
    kind: "toolResult".to_string(),
    title: "No Session Changes To Revert".to_string(),
    content: "This session has no saved project files left to revert.".to_string(),
    attributes: Some(std::collections::HashMap::from([
      ("action".to_string(), "thread.revertChanges".to_string()),
      ("receiptKind".to_string(), "sessionChangeRevert".to_string()),
      ("revertedCount".to_string(), "0".to_string()),
    ])),
  }
}
