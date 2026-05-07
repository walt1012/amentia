use std::collections::HashMap;

use pith_protocol::TimelineItem;

pub(crate) fn build_permission_denied_items(
  permission_sources: &HashMap<String, Vec<String>>,
  permission: &str,
  blocked_action: &str,
  workspace_name: &str,
  mut attributes: HashMap<String, String>,
) -> Vec<TimelineItem> {
  let granted_by = permission_sources
    .get(permission)
    .map(|plugins| plugins.join(", "))
    .unwrap_or_else(|| "none".to_string());
  attributes.insert("requiredPermission".to_string(), permission.to_string());
  attributes.insert("blockedAction".to_string(), blocked_action.to_string());
  attributes.insert("grantedBy".to_string(), granted_by.clone());

  vec![
    TimelineItem {
      kind: "warning".to_string(),
      title: "Plugin Permission Required".to_string(),
      content: format!(
        "Pith could not {blocked_action} in {workspace_name} because no enabled plugin grants `{permission}`."
      ),
      attributes: Some(attributes.clone()),
    },
    TimelineItem {
      kind: "assistantMessage".to_string(),
      title: "Assistant".to_string(),
      content: format!(
        "Enable a plugin that grants `{permission}` before asking Pith to {blocked_action}. Currently granted by: {granted_by}."
      ),
      attributes: Some(attributes),
    },
  ]
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn permission_denied_items_include_required_permission_metadata() {
    let items = build_permission_denied_items(
      &HashMap::new(),
      "shell.exec",
      "run a shell command",
      "pith",
      HashMap::from([("turnId".to_string(), "turn-1".to_string())]),
    );
    let attributes = items[0].attributes.as_ref().expect("attributes");

    assert_eq!(
      attributes.get("requiredPermission"),
      Some(&"shell.exec".to_string())
    );
    assert_eq!(attributes.get("grantedBy"), Some(&"none".to_string()));
  }
}
