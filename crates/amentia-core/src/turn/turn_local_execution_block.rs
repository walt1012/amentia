use std::collections::HashMap;

use amentia_protocol::TimelineItem;

pub(super) fn build_local_execution_blocked_items(
  permission: &str,
  blocked_action: &str,
  workspace_name: &str,
  mut attributes: HashMap<String, String>,
) -> Vec<TimelineItem> {
  attributes.insert("requiredPermission".to_string(), permission.to_string());
  attributes.insert("blockedAction".to_string(), blocked_action.to_string());

  vec![
    TimelineItem {
      kind: "warning".to_string(),
      title: "Action Blocked".to_string(),
      content: format!(
        "Amentia did not {blocked_action} in {workspace_name} because the selected action safety mode blocks this action."
      ),
      attributes: Some(attributes.clone()),
    },
    TimelineItem {
      kind: "assistantMessage".to_string(),
      title: "Assistant".to_string(),
      content: "Switch action safety mode before asking Amentia to make local project changes."
        .to_string(),
      attributes: Some(attributes),
    },
  ]
}
