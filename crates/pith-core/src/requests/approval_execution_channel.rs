use std::collections::HashMap;

use super::approval_execution_events::ApprovalExecutionEvents;
use super::approval_execution_timeline::{tool_start_item, warning_item};
use crate::approval_types::PendingApproval;

pub(super) fn append_approved_channel_message_execution(
  events: &mut ApprovalExecutionEvents,
  approval: &PendingApproval,
) {
  let attributes = channel_message_attributes(approval);
  events.push_item(tool_start_item(
    "channel_send",
    format!(
      "Preparing approved channel message for {}.",
      approval.relative_path
    ),
    Some(attributes.clone()),
  ));
  events.push_item(warning_item(
    "channel_send not sent",
    "Pith recorded the approval, but no channel adapter sender is connected yet. No external message was sent."
      .to_string(),
    Some(attributes),
  ));
}

fn channel_message_attributes(approval: &PendingApproval) -> HashMap<String, String> {
  let mut attributes = HashMap::from([
    ("approvalId".to_string(), approval.id.clone()),
    ("action".to_string(), approval.action.clone()),
    ("relativePath".to_string(), approval.relative_path.clone()),
  ]);
  if let Some(message) = approval.content.as_ref() {
    attributes.insert("channelMessage".to_string(), message.clone());
  }

  attributes
}
