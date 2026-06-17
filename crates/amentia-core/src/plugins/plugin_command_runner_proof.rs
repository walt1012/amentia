use std::collections::HashMap;

use amentia_plugin_host::PluginCommandEntry as HostPluginCommandEntry;

use super::plugin_command_runner_remote_write_proof::{
  insert_plugin_runner_remote_write_contract, plugin_runner_remote_write_contract_is_valid,
};
use super::plugin_command_runner_workflow_proof::{
  insert_plugin_runner_connector_workflow_contract,
  plugin_runner_connector_workflow_contract_is_valid,
};

pub(super) use super::plugin_command_runner_workflow_proof::{
  plugin_runner_expected_workflow_id, plugin_runner_items_include_workflow,
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

pub(super) fn plugin_runner_connector_service_is_bound(
  attributes: &HashMap<String, String>,
  service: &str,
) -> bool {
  !attributes.contains_key("pluginRunnerConnectorServices")
    || plugin_runner_target_service_is_bound(attributes, service)
}

pub(super) fn plugin_runner_target_service_is_bound(
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

pub(super) fn plugin_runner_attribute_value<'a>(
  attributes: &'a HashMap<String, String>,
  key: &str,
) -> Option<&'a str> {
  attributes
    .get(key)
    .map(|value| value.trim())
    .filter(|value| !value.is_empty())
}
