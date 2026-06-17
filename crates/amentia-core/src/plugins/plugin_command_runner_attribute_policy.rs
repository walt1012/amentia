use std::collections::HashMap;

pub(super) fn plugin_runner_owned_attributes(
  attributes: HashMap<String, String>,
) -> HashMap<String, String> {
  attributes
    .into_iter()
    .filter(|(key, _)| !plugin_runner_reserved_attribute(key))
    .collect()
}

pub(super) fn merge_plugin_runner_attributes(
  item_attributes: &mut HashMap<String, String>,
  runner_attributes: &HashMap<String, String>,
) {
  for (key, value) in runner_attributes {
    if plugin_runner_reserved_attribute(key) {
      item_attributes.insert(key.clone(), value.clone());
    } else {
      item_attributes
        .entry(key.clone())
        .or_insert_with(|| value.clone());
    }
  }
}

fn plugin_runner_reserved_attribute(key: &str) -> bool {
  matches!(
    key,
    "action"
      | "approvalId"
      | "commandId"
      | "commandInput"
      | "connectorId"
      | "connectorIds"
      | "connectorRepairHint"
      | "connectorStatus"
      | "decision"
      | "executionKind"
      | "memoryNoteId"
      | "memoryNoteTitle"
      | "memoryScope"
      | "permissionGate"
      | "pluginCommandRunId"
      | "pluginCommandStatus"
      | "pluginDisplayName"
      | "pluginId"
      | "pluginInstallStatus"
      | "pluginLifecycleStatus"
      | "pluginSourcePath"
      | "requiredPermission"
      | "runBlocker"
      | "runRepairHint"
      | "runStatus"
      | "remoteWriteStatus"
      | "sourcePath"
      | "streamingStatus"
      | "turnId"
  ) || (key.starts_with("connector") && !key.starts_with("connectorWorkflow"))
    || key.starts_with("mcp")
    || key.starts_with("pluginRunner")
    || key.starts_with("sandbox")
}

#[cfg(test)]
mod tests {
  use std::collections::HashMap;

  use super::{merge_plugin_runner_attributes, plugin_runner_owned_attributes};

  #[test]
  fn owned_attributes_drop_core_and_protocol_keys() {
    let attributes = HashMap::from([
      ("pluginId".to_string(), "spoofed-plugin".to_string()),
      ("sandboxMode".to_string(), "none".to_string()),
      ("connectorId".to_string(), "spoofed-connector".to_string()),
      (
        "connectorWorkflowId".to_string(),
        "notion.create-page".to_string(),
      ),
      ("customSignal".to_string(), "kept".to_string()),
    ]);

    let owned = plugin_runner_owned_attributes(attributes);

    assert!(!owned.contains_key("pluginId"));
    assert!(!owned.contains_key("sandboxMode"));
    assert!(!owned.contains_key("connectorId"));
    assert_eq!(
      owned.get("connectorWorkflowId").map(String::as_str),
      Some("notion.create-page")
    );
    assert_eq!(owned.get("customSignal").map(String::as_str), Some("kept"));
  }

  #[test]
  fn runner_attributes_override_reserved_keys_only() {
    let mut item_attributes = HashMap::from([
      (
        "pluginRunnerOutputStatus".to_string(),
        "spoofed".to_string(),
      ),
      ("customSignal".to_string(), "plugin-value".to_string()),
    ]);
    let runner_attributes = HashMap::from([
      (
        "pluginRunnerOutputStatus".to_string(),
        "envelope".to_string(),
      ),
      ("customSignal".to_string(), "runner-value".to_string()),
    ]);

    merge_plugin_runner_attributes(&mut item_attributes, &runner_attributes);

    assert_eq!(
      item_attributes
        .get("pluginRunnerOutputStatus")
        .map(String::as_str),
      Some("envelope")
    );
    assert_eq!(
      item_attributes.get("customSignal").map(String::as_str),
      Some("plugin-value")
    );
  }
}
