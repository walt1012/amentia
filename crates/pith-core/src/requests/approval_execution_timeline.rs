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

pub(super) fn tool_result_item(
  title: &str,
  content: String,
  attributes: Option<HashMap<String, String>>,
) -> TimelineItem {
  TimelineItem {
    kind: "toolResult".to_string(),
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

#[cfg(test)]
mod tests {
  use super::*;

  fn approval() -> PendingApproval {
    PendingApproval {
      id: "approval-1".to_string(),
      thread_id: "thread-1".to_string(),
      action: "write_file".to_string(),
      title: "Write README.md".to_string(),
      relative_path: "README.md".to_string(),
      content: Some("content".to_string()),
      command: None,
    }
  }

  #[test]
  fn approval_granted_item_keeps_decision_attributes() {
    let item = approval_granted_item(&approval());
    let attributes = item.attributes.expect("attributes");

    assert_eq!(item.kind, "approvalResolved");
    assert_eq!(item.title, "Approval Granted");
    assert_eq!(
      attributes.get("approvalId").map(String::as_str),
      Some("approval-1")
    );
    assert_eq!(
      attributes.get("decision").map(String::as_str),
      Some("approved")
    );
  }

  #[test]
  fn tool_result_item_uses_structured_tool_result_kind() {
    let item = tool_result_item(
      "write_file result",
      "Wrote 7 bytes.".to_string(),
      Some(HashMap::from([(
        "relativePath".to_string(),
        "README.md".to_string(),
      )])),
    );

    let attributes = item.attributes.expect("attributes");
    assert_eq!(item.kind, "toolResult");
    assert_eq!(item.title, "write_file result");
    assert_eq!(
      attributes.get("relativePath").map(String::as_str),
      Some("README.md")
    );
  }
}
