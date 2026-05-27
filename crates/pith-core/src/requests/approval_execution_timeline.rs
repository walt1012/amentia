use std::collections::HashMap;

use pith_protocol::TimelineItem;

use crate::approval_types::PendingApproval;

pub(super) fn approval_granted_item(approval: &PendingApproval) -> TimelineItem {
  let content = if approval.action == "run_plugin_command" {
    format!(
      "Approved plugin command {}.",
      approval
        .command
        .as_deref()
        .unwrap_or(&approval.relative_path)
    )
  } else if approval.action == "send_channel_message" {
    format!("Approved channel message for {}.", approval.relative_path)
  } else {
    format!(
      "Approved {} for {}.",
      approval.action, approval.relative_path
    )
  };
  TimelineItem {
    kind: "approvalResolved".to_string(),
    title: "Approval Granted".to_string(),
    content,
    attributes: Some(approval_resolution_attributes(approval, "approved")),
  }
}

pub(super) fn approval_denied_item(approval: &PendingApproval) -> TimelineItem {
  let content = if approval.action == "run_plugin_command" {
    format!(
      "Denied plugin command {}.",
      approval
        .command
        .as_deref()
        .unwrap_or(&approval.relative_path)
    )
  } else if approval.action == "send_channel_message" {
    format!("Denied channel message for {}.", approval.relative_path)
  } else {
    format!("Denied {} for {}.", approval.action, approval.relative_path)
  };
  TimelineItem {
    kind: "approvalResolved".to_string(),
    title: "Approval Denied".to_string(),
    content,
    attributes: Some(approval_resolution_attributes(approval, "denied")),
  }
}

fn approval_resolution_attributes(
  approval: &PendingApproval,
  decision: &str,
) -> HashMap<String, String> {
  let mut attributes = HashMap::from([
    ("approvalId".to_string(), approval.id.clone()),
    ("decision".to_string(), decision.to_string()),
    ("action".to_string(), approval.action.clone()),
    ("relativePath".to_string(), approval.relative_path.clone()),
  ]);
  if let Some(command_id) = approval.command.as_ref() {
    attributes.insert("commandId".to_string(), command_id.clone());
    if let Some(plugin_id) = plugin_id_from_command_id(command_id) {
      attributes.insert("pluginId".to_string(), plugin_id.to_string());
    }
  }
  if let Some(content) = approval.content.as_ref() {
    if approval.action == "send_channel_message" {
      attributes.insert("channelMessage".to_string(), content.clone());
    } else {
      attributes.insert("commandInput".to_string(), content.clone());
    }
  }

  attributes
}

fn plugin_id_from_command_id(command_id: &str) -> Option<&str> {
  command_id
    .split_once("::")
    .map(|(plugin_id, _)| plugin_id)
    .filter(|plugin_id| !plugin_id.is_empty())
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
    assert_eq!(
      attributes.get("action").map(String::as_str),
      Some("write_file")
    );
    assert_eq!(
      attributes.get("relativePath").map(String::as_str),
      Some("README.md")
    );
  }

  #[test]
  fn plugin_approval_resolution_keeps_command_context() {
    let mut approval = approval();
    approval.action = "run_plugin_command".to_string();
    approval.relative_path = "plugin:notion-runner".to_string();
    approval.command = Some("notion-runner::notion-runner.sync".to_string());
    approval.content = Some("sync today".to_string());

    let item = approval_denied_item(&approval);
    let attributes = item.attributes.expect("attributes");

    assert_eq!(
      attributes.get("commandId").map(String::as_str),
      Some("notion-runner::notion-runner.sync")
    );
    assert_eq!(
      attributes.get("pluginId").map(String::as_str),
      Some("notion-runner")
    );
    assert_eq!(
      attributes.get("commandInput").map(String::as_str),
      Some("sync today")
    );
  }

  #[test]
  fn channel_approval_resolution_keeps_message_context() {
    let mut approval = approval();
    approval.action = "send_channel_message".to_string();
    approval.relative_path = "channel:weixin-channel::weixin".to_string();
    approval.content = Some("Here is the short summary.".to_string());

    let item = approval_denied_item(&approval);
    let attributes = item.attributes.expect("attributes");

    assert_eq!(item.content, "Denied channel message for channel:weixin-channel::weixin.");
    assert_eq!(
      attributes.get("channelMessage").map(String::as_str),
      Some("Here is the short summary.")
    );
    assert!(!attributes.contains_key("commandInput"));
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
