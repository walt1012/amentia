use pith_plugin_host::PluginCommandEntry as HostPluginCommandEntry;

use super::plugin_command_execution::is_supported_plugin_command_execution;
use super::plugin_command_mcp_runner::mcp_runner_setup_blocker;
use super::plugin_command_permission_gate::plugin_command_permission_blocker;
use super::plugin_command_runner::stdio_runner_setup_blocker;
use super::plugin_connector_requirements::command_connector_requirements;
use crate::runtime_plugins::RuntimePluginState;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PluginCommandReadiness {
  pub(crate) run_status: String,
  pub(crate) run_blocker: Option<String>,
  pub(crate) required_connector_ids: Vec<String>,
}

impl PluginCommandReadiness {
  pub(crate) fn ready(required_connector_ids: Vec<String>) -> Self {
    Self {
      run_status: "ready".to_string(),
      run_blocker: None,
      required_connector_ids,
    }
  }

  pub(crate) fn blocked(
    run_status: &str,
    run_blocker: String,
    required_connector_ids: Vec<String>,
  ) -> Self {
    Self {
      run_status: run_status.to_string(),
      run_blocker: Some(run_blocker),
      required_connector_ids,
    }
  }

  pub(crate) fn is_ready(&self) -> bool {
    self.run_status == "ready"
  }
}

pub(crate) fn command_readiness(
  command: &HostPluginCommandEntry,
  plugin_state: &RuntimePluginState,
) -> PluginCommandReadiness {
  let connector_requirements = command_connector_requirements(command, plugin_state);
  let required_connectors = connector_requirements.connectors;
  let mut required_connector_ids = required_connectors
    .iter()
    .map(|connector| connector.connector_id.clone())
    .collect::<Vec<_>>();
  required_connector_ids.extend(connector_requirements.missing_connector_ids.clone());

  if command.execution.is_none() {
    return PluginCommandReadiness::blocked(
      "missingExecution",
      format!(
        "Plugin command `{}` requires an explicit execution contract.",
        command.command_id
      ),
      required_connector_ids,
    );
  }
  if !is_supported_plugin_command_execution(command) {
    return PluginCommandReadiness::blocked(
      "unsupportedExecution",
      format!(
        "Plugin command `{}` requires a supported execution contract.",
        command.command_id
      ),
      required_connector_ids,
    );
  }
  if let Some(connector_id) = connector_requirements.missing_connector_ids.first() {
    return PluginCommandReadiness::blocked(
      "missingConnector",
      format!(
        "Plugin command `{}` references connector `{}` that is not declared.",
        command.command_id, connector_id
      ),
      required_connector_ids,
    );
  }
  if let Some(run_blocker) =
    plugin_command_permission_blocker(command, connector_requirements.connector_backed)
  {
    return PluginCommandReadiness::blocked(
      "missingPermission",
      run_blocker,
      required_connector_ids,
    );
  }

  if let Some(connector) = required_connectors.iter().find(|connector| {
    plugin_state
      .connector_credential(&connector.connector_id)
      .is_none()
  }) {
    return PluginCommandReadiness::blocked(
      "needsConnectorAuth",
      format!(
        "Plugin command `{}` requires authorizing connector `{}` first.",
        command.command_id, connector.connector_id
      ),
      required_connector_ids,
    );
  }
  if let Some(run_blocker) = stdio_runner_setup_blocker(command) {
    return PluginCommandReadiness::blocked("runnerSetup", run_blocker, required_connector_ids);
  }
  if let Some(run_blocker) = mcp_runner_setup_blocker(command) {
    return PluginCommandReadiness::blocked("runnerSetup", run_blocker, required_connector_ids);
  }

  PluginCommandReadiness::ready(required_connector_ids)
}
