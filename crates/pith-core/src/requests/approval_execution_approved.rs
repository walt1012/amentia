use std::collections::HashMap;

use pith_memory::MemoryNote;
use pith_model_runtime::LocalModelRuntime;
use pith_plugin_host::PluginCatalogEntry;
use pith_protocol::WorkspaceSummary;

use super::approval_execution_events::ApprovalExecutionEvents;
use super::approval_execution_shell::append_approved_shell_execution;
use super::approval_execution_timeline::approval_granted_item;
use super::approval_execution_write::append_approved_write_execution;
use crate::approval_types::PendingApproval;

pub(super) fn execute_approved_approval(
  approval: &PendingApproval,
  workspace: &WorkspaceSummary,
  model_runtime: &LocalModelRuntime,
  memory_notes: &[MemoryNote],
  permission_sources: &HashMap<String, Vec<String>>,
  plugins: &[PluginCatalogEntry],
) -> ApprovalExecutionEvents {
  let mut events = ApprovalExecutionEvents::new();
  events.push_item(approval_granted_item(approval));

  match approval.action.as_str() {
    "write_file" => {
      append_approved_write_execution(&mut events, approval, workspace, permission_sources)
    }
    "run_shell" => append_approved_shell_execution(
      &mut events,
      approval,
      workspace,
      model_runtime,
      memory_notes,
      permission_sources,
      plugins,
    ),
    _ => {}
  }

  events
}
