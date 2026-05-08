use std::collections::HashMap;

use pith_protocol::WorkspaceSummary;

pub(super) fn workspace_tool_attributes(
  tool: &str,
  workspace: &WorkspaceSummary,
  extra: impl IntoIterator<Item = (String, String)>,
) -> HashMap<String, String> {
  let mut attributes = HashMap::from([
    ("tool".to_string(), tool.to_string()),
    (
      "workspaceDisplayName".to_string(),
      workspace.display_name.clone(),
    ),
    ("workspaceRootPath".to_string(), workspace.root_path.clone()),
  ]);
  attributes.extend(extra);
  attributes
}
