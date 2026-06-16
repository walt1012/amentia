use std::collections::HashMap;

use pith_plugin_host::{
  PluginCommandEntry as HostPluginCommandEntry, PluginConnectorWorkflowEntry,
};
use pith_protocol::TimelineItem;

use super::plugin_command_runner_contracts::{
  PLUGIN_RUNNER_CONNECTOR_WORKFLOW_CONTRACT, PLUGIN_RUNNER_CONNECTOR_WORKFLOW_STATUSES,
};
use super::plugin_command_runner_proof::{
  plugin_runner_attribute_value, plugin_runner_connector_service_is_bound,
};

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

pub(super) fn plugin_runner_connector_workflow_contract_is_valid(
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

pub(super) fn insert_plugin_runner_connector_workflow_contract(
  attributes: &mut HashMap<String, String>,
) {
  if plugin_runner_has_connector_workflow(attributes) {
    attributes.insert(
      "pluginRunnerConnectorWorkflowContract".to_string(),
      PLUGIN_RUNNER_CONNECTOR_WORKFLOW_CONTRACT.to_string(),
    );
  }
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

#[cfg(test)]
mod tests {
  use std::collections::HashMap;

  use pith_plugin_host::{
    PluginCommandEntry as HostPluginCommandEntry,
    PluginCommandEnvelopeEntry as HostPluginCommandEnvelopeEntry,
    PluginCommandExecutionEntry as HostPluginCommandExecutionEntry,
  };

  use super::{
    plugin_runner_connector_workflow_contract_is_valid, plugin_runner_expected_workflow_id,
  };

  #[test]
  fn workflow_contract_rejects_incomplete_metadata() {
    let command = test_workflow_command("notion.create-page");
    let mut attributes = HashMap::new();
    attributes.insert(
      "connectorWorkflowId".to_string(),
      "notion.create-page".to_string(),
    );
    attributes.insert("connectorWorkflowStatus".to_string(), "prepared".to_string());

    assert!(!plugin_runner_connector_workflow_contract_is_valid(&command, &attributes));
  }

  #[test]
  fn expected_workflow_id_prefers_execution_binding() {
    let command = test_workflow_command("notion.create-page");

    assert_eq!(
      plugin_runner_expected_workflow_id(&command),
      Some("notion.create-page")
    );
  }

  fn test_workflow_command(workflow_id: &str) -> HostPluginCommandEntry {
    HostPluginCommandEntry {
      command_id: "test-plugin::run".to_string(),
      title: "Run Test Plugin".to_string(),
      description: "Run a test plugin command.".to_string(),
      prompt: "Run the plugin.".to_string(),
      plugin_id: "test-plugin".to_string(),
      plugin_display_name: "Test Plugin".to_string(),
      permissions: vec![],
      source_path: "plugins/test-plugin/commands/run.json".to_string(),
      execution: Some(HostPluginCommandExecutionEntry {
        kind: "stdio.test".to_string(),
        driver: "stdio".to_string(),
        entrypoint: Some("bin/test-runner".to_string()),
        connector_ids: Some(vec!["notion".to_string()]),
        workflow_id: Some(workflow_id.to_string()),
        workflow: None,
        input: empty_envelope("pith.plugin.command.input"),
        output: empty_envelope("pith.plugin.command.output"),
      }),
      execution_kind: Some("stdio.test".to_string()),
      manifest_error: None,
      memory_note_title: None,
      memory_note_source: None,
      memory_note_tags: vec![],
    }
  }

  fn empty_envelope(envelope: &str) -> HostPluginCommandEnvelopeEntry {
    HostPluginCommandEnvelopeEntry {
      envelope: envelope.to_string(),
      fields: vec![],
    }
  }
}
