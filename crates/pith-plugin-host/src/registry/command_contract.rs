use crate::manifest::{
  PluginCommandEnvelopeFieldManifest, PluginCommandEnvelopeManifest, PluginCommandExecutionManifest,
};
use crate::types::{
  PluginCommandEnvelopeEntry, PluginCommandEnvelopeFieldEntry, PluginCommandExecutionEntry,
};

pub(super) fn command_execution_entry(
  execution: &PluginCommandExecutionManifest,
) -> Option<PluginCommandExecutionEntry> {
  let kind = execution.kind.trim().to_string();
  if kind.is_empty() {
    return None;
  }
  let driver = execution
    .driver
    .as_deref()
    .map(str::trim)
    .filter(|driver| !driver.is_empty())
    .map(str::to_string)
    .unwrap_or_else(|| default_execution_driver(&kind));

  Some(PluginCommandExecutionEntry {
    kind,
    driver,
    entrypoint: execution
      .entrypoint
      .as_deref()
      .map(str::trim)
      .filter(|entrypoint| !entrypoint.is_empty())
      .map(str::to_string),
    connector_ids: execution
      .connectors
      .as_deref()
      .map(normalized_connector_ids),
    input: command_envelope_entry(execution.input.as_ref(), default_input_envelope()),
    output: command_envelope_entry(execution.output.as_ref(), default_output_envelope()),
  })
}

fn normalized_connector_ids(connectors: &[String]) -> Vec<String> {
  connectors
    .iter()
    .map(|connector| connector.trim())
    .filter(|connector| !connector.is_empty())
    .map(str::to_string)
    .collect()
}

fn command_envelope_entry(
  envelope: Option<&PluginCommandEnvelopeManifest>,
  fallback: PluginCommandEnvelopeEntry,
) -> PluginCommandEnvelopeEntry {
  let Some(envelope) = envelope else {
    return fallback;
  };
  let envelope_name = envelope.envelope.trim();
  if envelope_name.is_empty() {
    return fallback;
  }

  PluginCommandEnvelopeEntry {
    envelope: envelope_name.to_string(),
    fields: envelope
      .fields
      .iter()
      .filter_map(command_envelope_field_entry)
      .collect(),
  }
}

fn command_envelope_field_entry(
  field: &PluginCommandEnvelopeFieldManifest,
) -> Option<PluginCommandEnvelopeFieldEntry> {
  let name = field.name.trim();
  let kind = field.kind.trim();
  if name.is_empty() || kind.is_empty() {
    return None;
  }

  Some(PluginCommandEnvelopeFieldEntry {
    name: name.to_string(),
    kind: kind.to_string(),
    required: field.required,
    description: field
      .description
      .as_deref()
      .map(str::trim)
      .filter(|description| !description.is_empty())
      .map(str::to_string),
  })
}

fn default_input_envelope() -> PluginCommandEnvelopeEntry {
  PluginCommandEnvelopeEntry {
    envelope: "pith.plugin.command.input".to_string(),
    fields: vec![
      command_envelope_field("threadId", "string", true, "Runtime thread identifier."),
      command_envelope_field("input", "text", false, "Optional user command input."),
      command_envelope_field(
        "workspace",
        "workspaceSummary",
        false,
        "Selected workspace context.",
      ),
    ],
  }
}

fn default_output_envelope() -> PluginCommandEnvelopeEntry {
  PluginCommandEnvelopeEntry {
    envelope: "pith.plugin.command.output".to_string(),
    fields: vec![
      command_envelope_field("items", "timelineItems", true, "Timeline items to append."),
      command_envelope_field(
        "memoryNotes",
        "memoryNotes",
        false,
        "Optional memory notes to store.",
      ),
    ],
  }
}

fn command_envelope_field(
  name: &str,
  kind: &str,
  required: bool,
  description: &str,
) -> PluginCommandEnvelopeFieldEntry {
  PluginCommandEnvelopeFieldEntry {
    name: name.to_string(),
    kind: kind.to_string(),
    required,
    description: Some(description.to_string()),
  }
}

fn default_execution_driver(kind: &str) -> String {
  kind
    .split_once('.')
    .map(|(driver, _)| driver)
    .filter(|driver| !driver.trim().is_empty())
    .unwrap_or("custom")
    .to_string()
}
