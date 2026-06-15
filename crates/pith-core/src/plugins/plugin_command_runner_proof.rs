use std::collections::HashMap;

use pith_plugin_host::{
  PluginCommandEntry as HostPluginCommandEntry, PluginConnectorWorkflowEntry,
};
use pith_protocol::TimelineItem;

use super::plugin_command_runner_contracts::{
  PLUGIN_RUNNER_CONNECTOR_WORKFLOW_CONTRACT, PLUGIN_RUNNER_CONNECTOR_WORKFLOW_STATUSES,
  PLUGIN_RUNNER_REMOTE_WRITE_COMPLETED_STAGE, PLUGIN_RUNNER_REMOTE_WRITE_CONTRACT,
  PLUGIN_RUNNER_REMOTE_WRITE_FAILED_BEFORE_PROOF_STAGE,
  PLUGIN_RUNNER_REMOTE_WRITE_INSPECTION_STAGE, PLUGIN_RUNNER_REMOTE_WRITE_STATUS_COMPLETED,
  PLUGIN_RUNNER_REMOTE_WRITE_STATUS_NOT_SENT, PLUGIN_RUNNER_REMOTE_WRITE_STATUS_PENDING,
  PLUGIN_RUNNER_REMOTE_WRITE_STATUS_UNCONFIRMED,
};

pub(super) fn plugin_runner_timeline_contracts_are_valid(
  command: &HostPluginCommandEntry,
  attributes: &HashMap<String, String>,
) -> bool {
  plugin_runner_remote_write_contract_is_valid(attributes)
    && plugin_runner_connector_workflow_contract_is_valid(command, attributes)
}

pub(super) fn insert_plugin_runner_timeline_contracts(attributes: &mut HashMap<String, String>) {
  insert_plugin_runner_remote_write_contract(attributes);
  insert_plugin_runner_connector_workflow_contract(attributes);
}

pub(super) fn plugin_runner_expected_workflow_id(command: &HostPluginCommandEntry) -> Option<&str> {
  if let Some(workflow) = plugin_runner_expected_workflow(command) {
    return Some(workflow.workflow_id.as_str());
  }

  command
    .execution
    .as_ref()
    .and_then(|execution| execution.workflow_id.as_deref())
    .map(str::trim)
    .filter(|workflow_id| !workflow_id.is_empty())
}

pub(super) fn plugin_runner_items_include_workflow(
  items: &[TimelineItem],
  workflow_id: &str,
) -> bool {
  items.iter().any(|item| {
    item
      .attributes
      .as_ref()
      .and_then(|attributes| plugin_runner_attribute_value(attributes, "connectorWorkflowId"))
      == Some(workflow_id)
  })
}

fn plugin_runner_connector_workflow_contract_is_valid(
  command: &HostPluginCommandEntry,
  attributes: &HashMap<String, String>,
) -> bool {
  if !plugin_runner_has_connector_workflow(attributes) {
    return true;
  }

  let required_keys = [
    "connectorWorkflowId",
    "connectorWorkflowService",
    "connectorWorkflowAction",
    "connectorWorkflowStage",
    "connectorWorkflowStatus",
    "connectorWorkflowTarget",
    "connectorWorkflowProof",
  ];
  if required_keys
    .iter()
    .any(|key| plugin_runner_attribute_value(attributes, key).is_none())
  {
    return false;
  }

  let Some(workflow_id) = plugin_runner_attribute_value(attributes, "connectorWorkflowId") else {
    return false;
  };
  let Some(service) = plugin_runner_attribute_value(attributes, "connectorWorkflowService") else {
    return false;
  };
  let Some(action) = plugin_runner_attribute_value(attributes, "connectorWorkflowAction") else {
    return false;
  };
  let Some(stage) = plugin_runner_attribute_value(attributes, "connectorWorkflowStage") else {
    return false;
  };
  let Some(status) = plugin_runner_attribute_value(attributes, "connectorWorkflowStatus") else {
    return false;
  };

  let Some(workflow) = plugin_runner_expected_workflow(command) else {
    if let Some(expected_workflow_id) = plugin_runner_expected_workflow_id(command) {
      if workflow_id != expected_workflow_id {
        return false;
      }
    }
    return PLUGIN_RUNNER_CONNECTOR_WORKFLOW_STATUSES.contains(&status)
      && plugin_runner_connector_service_is_bound(attributes, service);
  };
  if workflow_id != workflow.workflow_id.as_str()
    || service != workflow.service.as_str()
    || action != workflow.action.as_str()
    || !plugin_runner_workflow_list_contains(&workflow.stages, stage)
    || !plugin_runner_workflow_list_contains(&workflow.statuses, status)
  {
    return false;
  }
  if !plugin_runner_connector_service_is_bound(attributes, service) {
    return false;
  }

  true
}

fn plugin_runner_has_connector_workflow(attributes: &HashMap<String, String>) -> bool {
  attributes
    .keys()
    .any(|key| key.starts_with("connectorWorkflow"))
}

fn plugin_runner_expected_workflow(
  command: &HostPluginCommandEntry,
) -> Option<&PluginConnectorWorkflowEntry> {
  command
    .execution
    .as_ref()
    .and_then(|execution| execution.workflow.as_ref())
}

fn plugin_runner_workflow_list_contains(values: &[String], expected: &str) -> bool {
  values.iter().any(|value| value == expected)
}

fn plugin_runner_connector_service_is_bound(
  attributes: &HashMap<String, String>,
  service: &str,
) -> bool {
  !attributes.contains_key("pluginRunnerConnectorServices")
    || plugin_runner_target_service_is_bound(attributes, service)
}

fn plugin_runner_remote_write_contract_is_valid(attributes: &HashMap<String, String>) -> bool {
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

fn plugin_runner_target_service_is_bound(
  attributes: &HashMap<String, String>,
  target_service: &str,
) -> bool {
  plugin_runner_attribute_value(attributes, "pluginRunnerConnectorServices")
    .map(|services| {
      services
        .split(',')
        .map(str::trim)
        .any(|service| service == target_service)
    })
    .unwrap_or(false)
}

fn plugin_runner_attribute_value<'a>(
  attributes: &'a HashMap<String, String>,
  key: &str,
) -> Option<&'a str> {
  attributes
    .get(key)
    .map(|value| value.trim())
    .filter(|value| !value.is_empty())
}

fn insert_plugin_runner_remote_write_contract(attributes: &mut HashMap<String, String>) {
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

fn insert_plugin_runner_connector_workflow_contract(attributes: &mut HashMap<String, String>) {
  if plugin_runner_has_connector_workflow(attributes) {
    attributes.insert(
      "pluginRunnerConnectorWorkflowContract".to_string(),
      PLUGIN_RUNNER_CONNECTOR_WORKFLOW_CONTRACT.to_string(),
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
  if remote_write_stage == Some(PLUGIN_RUNNER_REMOTE_WRITE_FAILED_BEFORE_PROOF_STAGE) {
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
