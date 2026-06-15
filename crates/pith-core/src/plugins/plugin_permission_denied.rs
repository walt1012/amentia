use std::collections::HashMap;

use pith_protocol::TimelineItem;

pub(crate) fn build_permission_denied_items(
  permission_sources: &HashMap<String, Vec<String>>,
  permission: &str,
  blocked_action: &str,
  workspace_name: &str,
  mut attributes: HashMap<String, String>,
) -> Vec<TimelineItem> {
  let action_scope = if workspace_name == "the web" {
    "".to_string()
  } else {
    format!(" in {workspace_name}")
  };
  let granted_by = permission_sources
    .get(permission)
    .map(|plugins| plugins.join(", "))
    .unwrap_or_else(|| "none".to_string());
  let permission_label = readable_permission(permission);
  let recovery_hint = permission_recovery_hint(permission, blocked_action, &granted_by);
  attributes.insert("requiredPermission".to_string(), permission.to_string());
  attributes.insert(
    "requiredPermissionLabel".to_string(),
    permission_label.to_string(),
  );
  attributes.insert("blockedAction".to_string(), blocked_action.to_string());
  attributes.insert("grantedBy".to_string(), granted_by.clone());
  attributes.insert("permissionRecoveryHint".to_string(), recovery_hint.clone());

  vec![
    TimelineItem {
      kind: "warning".to_string(),
      title: "Plugin Permission Required".to_string(),
      content: format!(
        "Pith could not {blocked_action}{action_scope} because {permission_label} is not enabled."
      ),
      attributes: Some(attributes.clone()),
    },
    TimelineItem {
      kind: "assistantMessage".to_string(),
      title: "Assistant".to_string(),
      content: recovery_hint,
      attributes: Some(attributes),
    },
  ]
}

fn readable_permission(permission: &str) -> &str {
  match permission {
    "tool:web_search" => "Web Search",
    "network.outbound" => "network access",
    "file.read" => "project read permission",
    "file.write" => "project write permission",
    "shell.exec" => "shell execution permission",
    "mcp.connect" => "MCP connection permission",
    _ => permission,
  }
}

fn permission_recovery_hint(permission: &str, blocked_action: &str, granted_by: &str) -> String {
  match permission {
    "tool:web_search" => {
      concat!(
        "Enable Web Search from the readiness chip or Plugins > Permissions, ",
        "then retry the request."
      )
      .to_string()
    }
    _ => format!(
      "Enable a plugin that grants `{permission}` before asking Pith to {blocked_action}. Currently granted by: {granted_by}."
    ),
  }
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
    assert_eq!(
      attributes.get("requiredPermissionLabel"),
      Some(&"shell execution permission".to_string())
    );
    assert_eq!(attributes.get("grantedBy"), Some(&"none".to_string()));
  }

  #[test]
  fn permission_denied_items_do_not_repeat_web_scope() {
    let items = build_permission_denied_items(
      &HashMap::new(),
      "network.outbound",
      "search the web",
      "the web",
      HashMap::new(),
    );
    assert!(items[0]
      .content
      .contains("could not search the web because"));
    assert!(!items[0].content.contains("in the web"));
  }

  #[test]
  fn web_search_permission_denied_items_use_recovery_language() {
    let items = build_permission_denied_items(
      &HashMap::new(),
      "tool:web_search",
      "search the web",
      "the web",
      HashMap::new(),
    );
    let attributes = items[0].attributes.as_ref().expect("attributes");

    assert!(items[0].content.contains("Web Search is not enabled"));
    assert!(items[1].content.contains("Enable Web Search"));
    assert_eq!(
      attributes.get("requiredPermissionLabel"),
      Some(&"Web Search".to_string())
    );
    assert!(attributes
      .get("permissionRecoveryHint")
      .expect("recovery hint")
      .contains("Plugins > Permissions"));
  }
}
