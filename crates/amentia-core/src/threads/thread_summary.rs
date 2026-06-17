use amentia_protocol::{ApprovalRequest, WorkspaceSummary};
use anyhow::Result;

use crate::approval_state::approvals_for_thread;
use crate::runtime_context::RuntimeContext;
use crate::runtime_memory::RuntimeMemoryNoteDraft;
use crate::text_utils::truncate_text;
use crate::thread_state::StoredThread;

pub(crate) fn refresh_thread_summary_note(
  context: &mut RuntimeContext,
  thread_id: &str,
) -> Result<()> {
  let Some(thread) = context.thread_state.find(thread_id).cloned() else {
    return Ok(());
  };

  let pending_approvals = approvals_for_thread(context, thread_id);
  let workspace_snapshot = thread.workspace_cloned();
  let scope = thread
    .workspace()
    .map(|workspace| workspace.display_name.clone())
    .unwrap_or_else(|| "global".to_string());

  context.upsert_memory_note(
    format!("memory-thread-summary-{thread_id}"),
    RuntimeMemoryNoteDraft::new(
      format!("Thread summary: {}", thread.title()),
      build_thread_summary_body(&thread, workspace_snapshot.as_ref(), &pending_approvals),
      scope,
      "thread".to_string(),
      vec![
        "thread".to_string(),
        "summary".to_string(),
        thread_id.to_string(),
      ],
    ),
  )?;

  Ok(())
}

fn build_thread_summary_body(
  thread: &StoredThread,
  workspace: Option<&WorkspaceSummary>,
  pending_approvals: &[ApprovalRequest],
) -> String {
  let workspace_line = workspace
    .map(|workspace| format!("Workspace: {}.", workspace.display_name))
    .unwrap_or_else(|| "Workspace: unavailable.".to_string());
  let latest_user = thread
    .items()
    .iter()
    .rev()
    .find(|item| item.kind == "userMessage")
    .map(|item| truncate_text(&item.content, 180))
    .unwrap_or_else(|| "No user request captured yet.".to_string());
  let latest_assistant = thread
    .items()
    .iter()
    .rev()
    .find(|item| item.kind == "assistantMessage")
    .map(|item| truncate_text(&item.content, 180))
    .unwrap_or_else(|| "No assistant update captured yet.".to_string());
  let recent_activity = thread
    .items()
    .iter()
    .rev()
    .filter(|item| item.kind != "system")
    .take(4)
    .map(|item| item.title.clone())
    .collect::<Vec<_>>();
  let activity_line = if recent_activity.is_empty() {
    "Recent activity: none yet.".to_string()
  } else {
    format!("Recent activity: {}.", recent_activity.join(", "))
  };

  format!(
    "{workspace_line}\nStatus: {}.\nLast user request: {}.\nLatest assistant update: {}.\nPending approvals: {}.\n{}",
    thread.status(),
    latest_user,
    latest_assistant,
    pending_approvals.len(),
    activity_line
  )
}
