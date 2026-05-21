use pith_memory::MemoryEvent;
use pith_protocol::{TimelineItem, WorkspaceSummary};

use crate::approval_types::PendingApproval;
use crate::plugin_commands::PluginCommandOutput;
use crate::plugin_hooks::PluginHookMemoryCapture;
use crate::request_state::ApprovalExecutionOutput;
use crate::requests::approval_agent_context::ApprovalAgentContext;

pub(super) struct ApprovalExecutionEvents {
  items: Vec<TimelineItem>,
  memory_event: Option<MemoryEvent>,
  hook_memory_captures: Vec<PluginHookMemoryCapture>,
  approved_plugin_command_output: Option<PluginCommandOutput>,
}

impl ApprovalExecutionEvents {
  pub(super) fn new() -> Self {
    Self {
      items: vec![],
      memory_event: None,
      hook_memory_captures: vec![],
      approved_plugin_command_output: None,
    }
  }

  pub(super) fn push_item(&mut self, item: TimelineItem) {
    self.items.push(item);
  }

  pub(super) fn extend_items(&mut self, items: impl IntoIterator<Item = TimelineItem>) {
    self.items.extend(items);
  }

  pub(super) fn set_memory_event(&mut self, event: MemoryEvent) {
    self.memory_event = Some(event);
  }

  pub(super) fn extend_hook_memory_captures(
    &mut self,
    captures: impl IntoIterator<Item = PluginHookMemoryCapture>,
  ) {
    self.hook_memory_captures.extend(captures);
  }

  pub(super) fn set_approved_plugin_command_output(&mut self, output: PluginCommandOutput) {
    self.approved_plugin_command_output = Some(output);
  }

  pub(super) fn tag_agent_context(&mut self, agent_context: &ApprovalAgentContext) {
    agent_context.tag_items(&mut self.items);
  }

  pub(super) fn into_output(
    self,
    approval: PendingApproval,
    decision: String,
    workspace: WorkspaceSummary,
  ) -> ApprovalExecutionOutput {
    ApprovalExecutionOutput {
      approval,
      decision,
      workspace,
      items: self.items,
      memory_event: self.memory_event,
      hook_memory_captures: self.hook_memory_captures,
      approved_plugin_command_output: self.approved_plugin_command_output,
    }
  }
}
