use pith_memory::{MemoryEvent, MemoryNote};
use pith_model_runtime::LocalModelRuntime;
use pith_protocol::WorkspaceSummary;

use crate::approval_types::PendingApproval;
use crate::local_responses::summarize_denied_approval;

use super::approval_execution_events::ApprovalExecutionEvents;
use super::approval_execution_timeline::{approval_denied_item, assistant_item};

pub(super) fn execute_denied_approval(
  approval: &PendingApproval,
  workspace: &WorkspaceSummary,
  model_runtime: &LocalModelRuntime,
  memory_notes: &[MemoryNote],
) -> ApprovalExecutionEvents {
  let mut events = ApprovalExecutionEvents::new();
  events.set_memory_event(MemoryEvent::ApprovalDenied {
    title: approval.title.clone(),
    action: approval.action.clone(),
  });
  let (summary, summary_attributes) = summarize_denied_approval(
    model_runtime,
    memory_notes,
    &workspace.display_name,
    &approval.action,
    &approval.relative_path,
    approval.command.as_deref(),
  );
  events.push_item(approval_denied_item(approval));
  events.push_item(assistant_item(summary, Some(summary_attributes)));
  events
}
