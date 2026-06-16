use std::collections::HashMap;

use pith_plugin_host::PluginCommandEntry as HostPluginCommandEntry;
use pith_protocol::JsonRpcResponse;
use serde_json::{json, Value};

use super::plugin_command_input_contract::PluginCommandInputContractError;
use super::plugin_command_readiness::PluginCommandReadiness;

pub(crate) struct PluginCommandPreparationError {
  code: i32,
  message: String,
  data: Option<Value>,
}

impl PluginCommandPreparationError {
  pub(super) fn plain(code: i32, message: impl Into<String>) -> Self {
    Self {
      code,
      message: message.into(),
      data: None,
    }
  }

  pub(super) fn from_readiness(
    command: &HostPluginCommandEntry,
    readiness: PluginCommandReadiness,
  ) -> Self {
    let code = if readiness.run_status == "needsConnectorAuth" {
      -32058
    } else {
      -32053
    };
    let message = readiness.run_blocker.clone().unwrap_or_else(|| {
      format!(
        "Plugin command `{}` is not ready to run.",
        command.command_id
      )
    });
    let mut data = json!({
      "pluginId": &command.plugin_id,
      "commandId": &command.command_id,
      "sourcePath": &command.source_path,
      "runStatus": readiness.run_status,
      "runBlocker": readiness.run_blocker,
      "runRepairHint": readiness.run_repair_hint,
    });
    insert_connector_error_data(
      &mut data,
      readiness_connector_ids(&command.plugin_id, &readiness),
    );
    Self {
      code,
      message,
      data: Some(data),
    }
  }

  pub(super) fn from_input_contract(
    command: &HostPluginCommandEntry,
    error: PluginCommandInputContractError,
  ) -> Self {
    let message = error.message;
    Self {
      code: -32053,
      message: message.clone(),
      data: Some({
        let mut data = json!({
          "pluginId": &command.plugin_id,
          "commandId": &command.command_id,
          "sourcePath": &command.source_path,
          "runStatus": error.run_status,
          "runBlocker": message,
          "runRepairHint": error.run_repair_hint,
        });
        insert_connector_error_data(&mut data, declared_connector_ids(command));
        data
      }),
    }
  }

  pub(super) fn from_command_context(
    command: &HostPluginCommandEntry,
    code: i32,
    run_status: &'static str,
    message: impl Into<String>,
    run_repair_hint: &'static str,
  ) -> Self {
    let message = message.into();
    Self {
      code,
      message: message.clone(),
      data: Some(json!({
        "pluginId": &command.plugin_id,
        "commandId": &command.command_id,
        "sourcePath": &command.source_path,
        "runStatus": run_status,
        "runBlocker": message,
        "runRepairHint": run_repair_hint,
      })),
    }
  }

  pub(super) fn command_not_found(command_id: &str) -> Self {
    let message = "Plugin command not found";
    let repair_hint = concat!(
      "Refresh plugins, enable the plugin, ",
      "or select a command from the command panel."
    );
    Self {
      code: -32052,
      message: message.to_string(),
      data: Some(json!({
        "commandId": command_id,
        "runStatus": "commandNotFound",
        "runBlocker": message,
        "runRepairHint": repair_hint,
      })),
    }
  }

  pub(crate) fn into_response(self, request_id: Value) -> JsonRpcResponse {
    if let Some(data) = self.data {
      JsonRpcResponse::error_with_data(request_id, self.code, self.message, &data)
    } else {
      JsonRpcResponse::error(request_id, self.code, self.message)
    }
  }

  pub(crate) fn message(&self) -> &str {
    &self.message
  }

  pub(crate) fn route_failure_attributes(
    &self,
    command_id: &str,
    routing_reason: &str,
    input: Option<&str>,
  ) -> HashMap<String, String> {
    let mut attributes = HashMap::from([
      ("pluginCommandStatus".to_string(), "blocked".to_string()),
      (
        "pluginCommandRouting".to_string(),
        routing_reason.to_string(),
      ),
      ("commandId".to_string(), command_id.to_string()),
      ("errorCode".to_string(), self.code.to_string()),
    ]);
    if let Some(input) = input.map(str::trim).filter(|input| !input.is_empty()) {
      attributes.insert("commandInput".to_string(), input.to_string());
    }
    if let Some(data) = self.data.as_ref().and_then(|data| data.as_object()) {
      for (key, value) in data {
        match value {
          Value::String(text) if !text.is_empty() => {
            attributes.insert(key.to_string(), text.to_string());
          }
          Value::Null => {}
          value => {
            attributes.insert(key.to_string(), value.to_string());
          }
        }
      }
    }
    attributes
  }
}

fn insert_connector_error_data(data: &mut Value, connector_ids: Vec<String>) {
  if connector_ids.is_empty() {
    return;
  }

  data["connectorIds"] = json!(connector_ids.join(", "));
  if connector_ids.len() == 1 {
    data["connectorId"] = json!(&connector_ids[0]);
  }
}

fn readiness_connector_ids(plugin_id: &str, readiness: &PluginCommandReadiness) -> Vec<String> {
  if readiness.required_connector_ids.is_empty() {
    return qualify_connector_ids(plugin_id, &readiness.declared_connector_ids);
  }

  qualify_connector_ids(plugin_id, &readiness.required_connector_ids)
}

fn declared_connector_ids(command: &HostPluginCommandEntry) -> Vec<String> {
  let Some(connector_ids) = command
    .execution
    .as_ref()
    .and_then(|execution| execution.connector_ids.as_ref())
  else {
    return vec![];
  };

  qualify_connector_ids(&command.plugin_id, connector_ids)
}

fn qualify_connector_ids(plugin_id: &str, connector_ids: &[String]) -> Vec<String> {
  connector_ids
    .iter()
    .map(|connector_id| {
      if connector_id.contains("::") {
        connector_id.clone()
      } else {
        format!("{plugin_id}::{connector_id}")
      }
    })
    .collect()
}

#[cfg(test)]
mod tests {
  use pith_plugin_host::{
    PluginCommandEntry as HostPluginCommandEntry,
    PluginCommandEnvelopeEntry as HostPluginCommandEnvelopeEntry,
    PluginCommandExecutionEntry as HostPluginCommandExecutionEntry,
  };
  use serde_json::Value;

  use super::PluginCommandPreparationError;
  use crate::plugins::plugin_command_input_contract::PluginCommandInputContractError;

  #[test]
  fn route_failure_attributes_keep_source_path_and_input() {
    let command = test_command(None);
    let error = PluginCommandPreparationError::from_input_contract(
      &command,
      PluginCommandInputContractError::new(
        "Missing input.".to_string(),
        "missingInput",
        "Run with input.",
      ),
    );

    let attributes = error.route_failure_attributes(
      "test-plugin::run",
      "slashPluginCommand",
      Some("retry this input"),
    );

    assert_eq!(
      attributes.get("sourcePath").map(String::as_str),
      Some("plugins/test-plugin/commands/run.json")
    );
    assert_eq!(
      attributes.get("pluginCommandStatus").map(String::as_str),
      Some("blocked")
    );
    assert_eq!(
      attributes.get("commandInput").map(String::as_str),
      Some("retry this input")
    );
    assert_eq!(
      attributes.get("runRepairHint").map(String::as_str),
      Some("Run with input.")
    );
  }

  #[test]
  fn input_contract_failure_keeps_connector_ids_for_repair() {
    let command = test_command(Some(test_connector_execution()));
    let error = PluginCommandPreparationError::from_input_contract(
      &command,
      PluginCommandInputContractError::new(
        "Missing connection input.".to_string(),
        "missingConnectorInput",
        "Declare and authorize a connector.",
      ),
    );

    let data = error
      .data
      .as_ref()
      .and_then(|data| data.as_object())
      .expect("error data");
    assert_eq!(
      data.get("connectorId").and_then(Value::as_str),
      Some("test-plugin::notion")
    );
    assert_eq!(
      data.get("connectorIds").and_then(Value::as_str),
      Some("test-plugin::notion")
    );
  }

  fn test_command(
    execution: Option<HostPluginCommandExecutionEntry>,
  ) -> HostPluginCommandEntry {
    HostPluginCommandEntry {
      command_id: "test-plugin::run".to_string(),
      title: "Run Test Plugin".to_string(),
      description: "Run a test plugin command.".to_string(),
      prompt: "Run the plugin.".to_string(),
      plugin_id: "test-plugin".to_string(),
      plugin_display_name: "Test Plugin".to_string(),
      permissions: vec![],
      source_path: "plugins/test-plugin/commands/run.json".to_string(),
      execution,
      execution_kind: Some("stdio.test".to_string()),
      manifest_error: None,
      memory_note_title: None,
      memory_note_source: None,
      memory_note_tags: vec![],
    }
  }

  fn test_connector_execution() -> HostPluginCommandExecutionEntry {
    HostPluginCommandExecutionEntry {
      kind: "stdio.test".to_string(),
      driver: "stdio".to_string(),
      entrypoint: Some("runner.sh".to_string()),
      connector_ids: Some(vec!["notion".to_string()]),
      workflow_id: Some("notion.create-page".to_string()),
      workflow: None,
      input: HostPluginCommandEnvelopeEntry {
        envelope: "pith.plugin.command.input".to_string(),
        fields: vec![],
      },
      output: HostPluginCommandEnvelopeEntry {
        envelope: "pith.plugin.command.output".to_string(),
        fields: vec![],
      },
    }
  }
}
