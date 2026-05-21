use std::collections::HashMap;

use pith_protocol::WorkspaceSummary;

pub(crate) const LOCAL_TOOL_SCHEMA: &str = "pith.localTool.v1";

pub(crate) fn local_tool_attributes(
  tool_kind: &str,
  tool_name: &str,
  extra: impl IntoIterator<Item = (String, String)>,
) -> HashMap<String, String> {
  let mut attributes = HashMap::from([
    ("tool".to_string(), tool_name.to_string()),
    ("toolName".to_string(), tool_name.to_string()),
    ("toolKind".to_string(), tool_kind.to_string()),
    ("toolSchema".to_string(), LOCAL_TOOL_SCHEMA.to_string()),
  ]);
  attributes.extend(extra);
  attributes
}

pub(crate) fn workspace_tool_attributes(
  tool: &str,
  workspace: &WorkspaceSummary,
  extra: impl IntoIterator<Item = (String, String)>,
) -> HashMap<String, String> {
  let mut attributes = local_tool_attributes(workspace_tool_kind(tool), tool, extra);
  attributes.extend([
    (
      "workspaceDisplayName".to_string(),
      workspace.display_name.clone(),
    ),
    ("workspaceRootPath".to_string(), workspace.root_path.clone()),
  ]);
  attributes
}

pub(crate) fn web_tool_attributes(
  tool: &str,
  extra: impl IntoIterator<Item = (String, String)>,
) -> HashMap<String, String> {
  local_tool_attributes("web", tool, extra)
}

fn workspace_tool_kind(tool: &str) -> &'static str {
  match tool {
    "read_file" | "write_file" | "generate_diff" => "file",
    "search_files" => "search",
    "run_shell" => "shell",
    _ => "workspace",
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn local_tool_attributes_keep_legacy_tool_name_and_schema() {
    let attributes = local_tool_attributes(
      "web",
      "web_search",
      [("query".to_string(), "pith release".to_string())],
    );

    assert_eq!(attributes.get("tool").map(String::as_str), Some("web_search"));
    assert_eq!(
      attributes.get("toolName").map(String::as_str),
      Some("web_search")
    );
    assert_eq!(attributes.get("toolKind").map(String::as_str), Some("web"));
    assert_eq!(
      attributes.get("toolSchema").map(String::as_str),
      Some("pith.localTool.v1")
    );
    assert_eq!(
      attributes.get("query").map(String::as_str),
      Some("pith release")
    );
  }

  #[test]
  fn workspace_tool_attributes_infer_core_tool_kind() {
    let workspace = WorkspaceSummary {
      display_name: "Pith".to_string(),
      root_path: "/workspace/pith".to_string(),
    };

    let read_attributes = workspace_tool_attributes("read_file", &workspace, []);
    let shell_attributes = workspace_tool_attributes("run_shell", &workspace, []);
    let search_attributes = workspace_tool_attributes("search_files", &workspace, []);

    assert_eq!(
      read_attributes.get("toolKind").map(String::as_str),
      Some("file")
    );
    assert_eq!(
      shell_attributes.get("toolKind").map(String::as_str),
      Some("shell")
    );
    assert_eq!(
      search_attributes.get("toolKind").map(String::as_str),
      Some("search")
    );
  }
}
