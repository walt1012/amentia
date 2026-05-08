use std::collections::HashMap;

use pith_protocol::TimelineItem;

use crate::approval_types::PendingApproval;

pub(super) fn approval_granted_item(approval: &PendingApproval) -> TimelineItem {
  TimelineItem {
    kind: "approvalResolved".to_string(),
    title: "Approval Granted".to_string(),
    content: format!(
      "Approved {} for {}.",
      approval.action, approval.relative_path
    ),
    attributes: Some(HashMap::from([
      ("approvalId".to_string(), approval.id.clone()),
      ("decision".to_string(), "approved".to_string()),
    ])),
  }
}

pub(super) fn approval_denied_item(approval: &PendingApproval) -> TimelineItem {
  TimelineItem {
    kind: "approvalResolved".to_string(),
    title: "Approval Denied".to_string(),
    content: format!("Denied {} for {}.", approval.action, approval.relative_path),
    attributes: Some(HashMap::from([
      ("approvalId".to_string(), approval.id.clone()),
      ("decision".to_string(), "denied".to_string()),
    ])),
  }
}

pub(super) fn tool_start_item(
  title: &str,
  content: String,
  attributes: Option<HashMap<String, String>>,
) -> TimelineItem {
  TimelineItem {
    kind: "toolStart".to_string(),
    title: title.to_string(),
    content,
    attributes,
  }
}

pub(super) fn warning_item(
  title: &str,
  content: String,
  attributes: Option<HashMap<String, String>>,
) -> TimelineItem {
  TimelineItem {
    kind: "warning".to_string(),
    title: title.to_string(),
    content,
    attributes,
  }
}

pub(super) fn assistant_item(
  content: String,
  attributes: Option<HashMap<String, String>>,
) -> TimelineItem {
  TimelineItem {
    kind: "assistantMessage".to_string(),
    title: "Assistant".to_string(),
    content,
    attributes,
  }
}
