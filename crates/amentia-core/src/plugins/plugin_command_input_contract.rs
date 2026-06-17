use amentia_plugin_host::{
  PluginCommandEntry as HostPluginCommandEntry, PluginCommandEnvelopeFieldEntry,
};
use amentia_protocol::WorkspaceSummary;

use super::plugin_command_types::PluginConnectorExecutionRef;

pub(super) fn validate_plugin_command_input_contract(
  command: &HostPluginCommandEntry,
  workspace: Option<&WorkspaceSummary>,
  input: Option<&str>,
  connector_refs: &[PluginConnectorExecutionRef],
) -> std::result::Result<(), PluginCommandInputContractError> {
  let Some(execution) = command.execution.as_ref() else {
    return Ok(());
  };

  if let Some(field) = input.and_then(|_| {
    execution
      .input
      .fields
      .iter()
      .find(|field| field.name.trim() == "input" && !input_field_accepts_plain_text(field))
  }) {
    return Err(PluginCommandInputContractError::new(
      format!(
        "Plugin command `{}` declares input field `input` with unsupported kind `{}` for plain command input.",
        command.command_id, field.kind
      ),
      "unsupportedInputField",
      "Use a text or string `input` field for plain input, or omit plain input until structured command inputs are supported.",
    ));
  }

  for field in execution.input.fields.iter().filter(|field| field.required) {
    match field.name.trim() {
      "threadId" | "commandId" | "envelope" => {}
      "input" if !input_field_accepts_plain_text(field) => {
        return Err(PluginCommandInputContractError::new(
          format!(
            "Plugin command `{}` requires unsupported input field `input` of kind `{}`.",
            command.command_id, field.kind
          ),
          "unsupportedInputField",
          "Use a text or string `input` field for plain input, or make the structured input optional.",
        ));
      }
      "input" if input.is_some() => {}
      "input" => {
        return Err(PluginCommandInputContractError::new(
          format!(
            "Plugin command `{}` requires command input field `input`.",
            command.command_id
          ),
          "missingInput",
          "Run the command with input or make the `input` field optional.",
        ));
      }
      "workspace" if workspace.is_some() => {}
      "workspace" => {
        return Err(PluginCommandInputContractError::new(
          format!(
            "Plugin command `{}` requires an open workspace for input field `workspace`.",
            command.command_id
          ),
          "missingWorkspaceInput",
          "Open a project before running this action or make the field optional.",
        ));
      }
      "connectors" if !connector_refs.is_empty() => {}
      "connectors" => {
        return Err(PluginCommandInputContractError::new(
          format!(
            "Plugin command `{}` requires connector input field `connectors`, but no connector context is available.",
            command.command_id
          ),
          "missingConnectorInput",
          "Declare and authorize a connector, or make the connector input optional.",
        ));
      }
      field_name => {
        return Err(PluginCommandInputContractError::new(
          format!(
            "Plugin command `{}` requires unsupported input field `{field_name}`.",
            command.command_id
          ),
          "unsupportedInputField",
          "Use supported required fields: threadId, commandId, envelope, input, workspace, or connectors.",
        ));
      }
    }
  }

  Ok(())
}

fn input_field_accepts_plain_text(field: &PluginCommandEnvelopeFieldEntry) -> bool {
  matches!(
    field.kind.trim().to_ascii_lowercase().as_str(),
    "text" | "string"
  )
}

pub(super) struct PluginCommandInputContractError {
  pub(super) message: String,
  pub(super) run_status: &'static str,
  pub(super) run_repair_hint: &'static str,
}

impl PluginCommandInputContractError {
  pub(super) fn new(
    message: String,
    run_status: &'static str,
    run_repair_hint: &'static str,
  ) -> Self {
    Self {
      message,
      run_status,
      run_repair_hint,
    }
  }
}

#[cfg(test)]
mod tests {
  use amentia_plugin_host::{
    PluginCommandEntry as HostPluginCommandEntry,
    PluginCommandEnvelopeEntry as HostPluginCommandEnvelopeEntry,
    PluginCommandEnvelopeFieldEntry as HostPluginCommandEnvelopeFieldEntry,
    PluginCommandExecutionEntry as HostPluginCommandExecutionEntry,
  };

  use super::validate_plugin_command_input_contract;

  #[test]
  fn input_contract_rejects_structured_required_input_field() {
    let command = test_command_with_execution(test_input_execution("object", true));

    let error =
      validate_plugin_command_input_contract(&command, None, None, &[]).expect_err("input error");

    assert_eq!(error.run_status, "unsupportedInputField");
    assert!(error.message.contains("unsupported input field `input`"));
    assert!(error.run_repair_hint.contains("text or string"));
  }

  #[test]
  fn input_contract_rejects_plain_input_for_structured_optional_field() {
    let command = test_command_with_execution(test_input_execution("json", false));

    let error = validate_plugin_command_input_contract(&command, None, Some("plain input"), &[])
      .expect_err("input error");

    assert_eq!(error.run_status, "unsupportedInputField");
    assert!(error.message.contains("unsupported kind `json`"));
    assert!(error.run_repair_hint.contains("structured command inputs"));
  }

  fn test_command_with_execution(
    execution: HostPluginCommandExecutionEntry,
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
      execution: Some(execution),
      execution_kind: Some("stdio.test".to_string()),
      manifest_error: None,
      memory_note_title: None,
      memory_note_source: None,
      memory_note_tags: vec![],
    }
  }

  fn test_input_execution(
    input_kind: &str,
    input_required: bool,
  ) -> HostPluginCommandExecutionEntry {
    HostPluginCommandExecutionEntry {
      kind: "stdio.test".to_string(),
      driver: "stdio".to_string(),
      entrypoint: Some("runner.sh".to_string()),
      connector_ids: None,
      workflow_id: None,
      workflow: None,
      input: HostPluginCommandEnvelopeEntry {
        envelope: "amentia.plugin.command.input".to_string(),
        fields: vec![HostPluginCommandEnvelopeFieldEntry {
          name: "input".to_string(),
          kind: input_kind.to_string(),
          required: input_required,
          description: None,
        }],
      },
      output: HostPluginCommandEnvelopeEntry {
        envelope: "amentia.plugin.command.output".to_string(),
        fields: vec![],
      },
    }
  }
}
