use pith_protocol::{TimelineItem, WorkspaceSummary};

use super::turn_tool_provenance::workspace_tool_attributes;

pub(super) fn workspace_tool_start_item(
  tool: &str,
  content: String,
  workspace: &WorkspaceSummary,
  extra: impl IntoIterator<Item = (String, String)>,
) -> TimelineItem {
  TimelineItem {
    kind: "toolStart".to_string(),
    title: tool.to_string(),
    content,
    attributes: Some(workspace_tool_attributes(tool, workspace, extra)),
  }
}

pub(super) fn workspace_tool_result_item(
  tool: &str,
  content: String,
  workspace: &WorkspaceSummary,
  extra: impl IntoIterator<Item = (String, String)>,
) -> TimelineItem {
  TimelineItem {
    kind: "toolResult".to_string(),
    title: format!("{tool} result"),
    content,
    attributes: Some(workspace_tool_attributes(tool, workspace, extra)),
  }
}

pub(super) fn workspace_tool_warning_item(
  tool: &str,
  title: String,
  content: String,
  workspace: &WorkspaceSummary,
  extra: impl IntoIterator<Item = (String, String)>,
) -> TimelineItem {
  TimelineItem {
    kind: "warning".to_string(),
    title,
    content,
    attributes: Some(workspace_tool_attributes(tool, workspace, extra)),
  }
}

pub(super) fn workspace_diff_artifact_item(
  content: String,
  workspace: &WorkspaceSummary,
  extra: impl IntoIterator<Item = (String, String)>,
) -> TimelineItem {
  TimelineItem {
    kind: "diffArtifact".to_string(),
    title: "Diff Preview".to_string(),
    content,
    attributes: Some(workspace_tool_attributes("generate_diff", workspace, extra)),
  }
}

pub(super) fn workspace_tool_failed_items(
  tool: &str,
  error: String,
  assistant_message: String,
  workspace: &WorkspaceSummary,
  extra: impl IntoIterator<Item = (String, String)>,
) -> Vec<TimelineItem> {
  vec![
    TimelineItem {
      kind: "warning".to_string(),
      title: format!("{tool} failed"),
      content: error,
      attributes: Some(workspace_tool_attributes(tool, workspace, extra)),
    },
    TimelineItem {
      kind: "assistantMessage".to_string(),
      title: "Assistant".to_string(),
      content: assistant_message,
      attributes: None,
    },
  ]
}

#[cfg(test)]
mod tests {
  use super::*;

  fn workspace() -> WorkspaceSummary {
    WorkspaceSummary {
      display_name: "Pith".to_string(),
      root_path: "/workspace/pith".to_string(),
    }
  }

  #[test]
  fn workspace_tool_start_item_uses_shared_provenance() {
    let item = workspace_tool_start_item(
      "read_file",
      "README.md".to_string(),
      &workspace(),
      [("relativePath".to_string(), "README.md".to_string())],
    );

    let attributes = item.attributes.expect("attributes");
    assert_eq!(item.kind, "toolStart");
    assert_eq!(item.title, "read_file");
    assert_eq!(attributes.get("tool").map(String::as_str), Some("read_file"));
    assert_eq!(
      attributes.get("workspaceDisplayName").map(String::as_str),
      Some("Pith")
    );
    assert_eq!(
      attributes.get("relativePath").map(String::as_str),
      Some("README.md")
    );
  }

  #[test]
  fn workspace_tool_failed_items_keep_warning_and_assistant_message_together() {
    let items = workspace_tool_failed_items(
      "search_files",
      "search failed".to_string(),
      "Try again.".to_string(),
      &workspace(),
      [("query".to_string(), "model".to_string())],
    );

    assert_eq!(items.len(), 2);
    assert_eq!(items[0].kind, "warning");
    assert_eq!(items[0].title, "search_files failed");
    assert_eq!(items[1].kind, "assistantMessage");
    assert_eq!(items[1].content, "Try again.");
  }

  #[test]
  fn workspace_diff_artifact_item_keeps_generate_diff_provenance() {
    let item = workspace_diff_artifact_item(
      "diff --git a/README.md b/README.md".to_string(),
      &workspace(),
      [("relativePath".to_string(), "README.md".to_string())],
    );

    let attributes = item.attributes.expect("attributes");
    assert_eq!(item.kind, "diffArtifact");
    assert_eq!(item.title, "Diff Preview");
    assert_eq!(
      attributes.get("tool").map(String::as_str),
      Some("generate_diff")
    );
    assert_eq!(
      attributes.get("relativePath").map(String::as_str),
      Some("README.md")
    );
  }
}
