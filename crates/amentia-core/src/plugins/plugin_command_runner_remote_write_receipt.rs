use std::collections::HashMap;

use super::plugin_command_runner_contracts::{
  PLUGIN_RUNNER_REMOTE_WRITE_COMPLETED_STAGE, PLUGIN_RUNNER_REMOTE_WRITE_CONTRACT,
  PLUGIN_RUNNER_REMOTE_WRITE_FAILED_BEFORE_CONFIRMATION_STAGE,
  PLUGIN_RUNNER_REMOTE_WRITE_INSPECTION_STAGE, PLUGIN_RUNNER_REMOTE_WRITE_STATUS_COMPLETED,
  PLUGIN_RUNNER_REMOTE_WRITE_STATUS_NOT_SENT, PLUGIN_RUNNER_REMOTE_WRITE_STATUS_PENDING,
  PLUGIN_RUNNER_REMOTE_WRITE_STATUS_UNCONFIRMED,
};
use super::plugin_command_runner_timeline_receipt::{
  plugin_runner_attribute_value, plugin_runner_target_service_is_bound,
};

pub(super) fn plugin_runner_remote_write_contract_is_valid(
  attributes: &HashMap<String, String>,
) -> bool {
  let remote_write = attributes
    .get("remoteWrite")
    .map(|value| value.trim())
    .unwrap_or_default();

  match remote_write {
    "" | "false" => {
      plugin_runner_attribute_value(attributes, "remoteWriteStage")
        != Some(PLUGIN_RUNNER_REMOTE_WRITE_COMPLETED_STAGE)
    }
    "true" => {
      let Some(target_service) = plugin_runner_attribute_value(attributes, "targetService") else {
        return false;
      };
      plugin_runner_attribute_value(attributes, "remoteWriteStage")
        == Some(PLUGIN_RUNNER_REMOTE_WRITE_COMPLETED_STAGE)
        && plugin_runner_target_service_is_bound(attributes, target_service)
        && plugin_runner_attribute_value(attributes, "targetTool").is_some()
    }
    _ => false,
  }
}

pub(super) fn insert_plugin_runner_remote_write_contract(attributes: &mut HashMap<String, String>) {
  if attributes.contains_key("remoteWrite") || attributes.contains_key("remoteWriteStage") {
    attributes.insert(
      "pluginRunnerRemoteWriteContract".to_string(),
      PLUGIN_RUNNER_REMOTE_WRITE_CONTRACT.to_string(),
    );
    attributes.insert(
      "remoteWriteStatus".to_string(),
      plugin_runner_remote_write_status(attributes).to_string(),
    );
  }
}

fn plugin_runner_remote_write_status(attributes: &HashMap<String, String>) -> &'static str {
  let remote_write = attributes
    .get("remoteWrite")
    .map(|value| value.trim())
    .unwrap_or_default();
  let remote_write_stage = plugin_runner_attribute_value(attributes, "remoteWriteStage");

  if remote_write == "true"
    && remote_write_stage == Some(PLUGIN_RUNNER_REMOTE_WRITE_COMPLETED_STAGE)
  {
    return PLUGIN_RUNNER_REMOTE_WRITE_STATUS_COMPLETED;
  }
  if remote_write_stage == Some(PLUGIN_RUNNER_REMOTE_WRITE_FAILED_BEFORE_CONFIRMATION_STAGE) {
    return PLUGIN_RUNNER_REMOTE_WRITE_STATUS_UNCONFIRMED;
  }
  if remote_write == "false" {
    return PLUGIN_RUNNER_REMOTE_WRITE_STATUS_NOT_SENT;
  }
  if remote_write_stage == Some(PLUGIN_RUNNER_REMOTE_WRITE_INSPECTION_STAGE) {
    return PLUGIN_RUNNER_REMOTE_WRITE_STATUS_NOT_SENT;
  }
  PLUGIN_RUNNER_REMOTE_WRITE_STATUS_PENDING
}

#[cfg(test)]
mod tests {
  use std::collections::HashMap;

  use super::{
    insert_plugin_runner_remote_write_contract, plugin_runner_remote_write_contract_is_valid,
  };

  #[test]
  fn remote_write_requires_bound_service_for_completed_writes() {
    let mut attributes = HashMap::new();
    attributes.insert("remoteWrite".to_string(), "true".to_string());
    attributes.insert("remoteWriteStage".to_string(), "completed".to_string());
    attributes.insert("targetService".to_string(), "notion".to_string());
    attributes.insert("targetTool".to_string(), "notion.updatePage".to_string());

    assert!(!plugin_runner_remote_write_contract_is_valid(&attributes));

    attributes.insert(
      "pluginRunnerConnectorServices".to_string(),
      "notion".to_string(),
    );

    assert!(plugin_runner_remote_write_contract_is_valid(&attributes));
  }

  #[test]
  fn remote_write_contract_owns_failed_confirmation_status() {
    let mut attributes = HashMap::new();
    attributes.insert("remoteWrite".to_string(), "false".to_string());
    attributes.insert(
      "remoteWriteStage".to_string(),
      "failedBeforeProof".to_string(),
    );
    attributes.insert("remoteWriteStatus".to_string(), "notSent".to_string());

    insert_plugin_runner_remote_write_contract(&mut attributes);

    assert_eq!(
      attributes.get("remoteWriteStatus").map(String::as_str),
      Some("unconfirmed")
    );
    assert_eq!(
      attributes
        .get("pluginRunnerRemoteWriteContract")
        .map(String::as_str),
      Some("amentia.connectorRemoteWrite.v1")
    );
  }

  #[test]
  fn completed_stage_without_remote_write_is_invalid() {
    let mut attributes = HashMap::new();
    attributes.insert("remoteWrite".to_string(), "false".to_string());
    attributes.insert("remoteWriteStage".to_string(), "completed".to_string());

    assert!(!plugin_runner_remote_write_contract_is_valid(&attributes));
  }
}
