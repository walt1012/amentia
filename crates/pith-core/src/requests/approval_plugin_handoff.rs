use std::collections::HashMap;

use pith_protocol::TimelineItem;

use crate::plugin_commands::PluginCommandOutput;

const HANDOFF_PREVIEW_LIMIT: usize = 360;

pub(super) fn build_approved_plugin_handoff(
  output: &PluginCommandOutput,
) -> Option<TimelineItem> {
  let observation = primary_observation(&output.items)?;
  let observation_title = observation.title.trim();
  let observation_preview = bounded_preview(&observation.content);
  let mut attributes = HashMap::from([
    ("pluginId".to_string(), output.command.plugin_id.clone()),
    ("commandId".to_string(), output.command.command_id.clone()),
    (
      "pluginCommandHandoff".to_string(),
      "approvedPluginCommand".to_string(),
    ),
    (
      "pluginCommandObservationKind".to_string(),
      observation.kind.clone(),
    ),
    (
      "pluginCommandObservationTitle".to_string(),
      observation_title.to_string(),
    ),
  ]);
  if let Some(execution_kind) = output.command.execution_kind.as_ref() {
    attributes.insert("executionKind".to_string(), execution_kind.clone());
  }
  copy_connector_attributes(&mut attributes, observation);

  Some(TimelineItem {
    kind: "assistantMessage".to_string(),
    title: "Assistant".to_string(),
    content: format!(
      "{} completed. Observation: {}. {}",
      output.command.title, observation_title, observation_preview
    ),
    attributes: Some(attributes),
  })
}

fn primary_observation(items: &[TimelineItem]) -> Option<&TimelineItem> {
  items
    .iter()
    .rev()
    .find(|item| item.kind == "pluginResult" || item.kind == "toolResult")
}

fn bounded_preview(content: &str) -> String {
  let trimmed = content.trim();
  let mut preview = trimmed
    .chars()
    .take(HANDOFF_PREVIEW_LIMIT)
    .collect::<String>();
  if trimmed.chars().count() > HANDOFF_PREVIEW_LIMIT {
    preview.push_str("...");
  }
  preview
}

fn copy_connector_attributes(
  attributes: &mut HashMap<String, String>,
  observation: &TimelineItem,
) {
  let Some(observation_attributes) = observation.attributes.as_ref() else {
    return;
  };
  for key in ["connectorId", "connectorIds", "connectorServices"] {
    if let Some(value) = observation_attributes.get(key) {
      attributes.insert(key.to_string(), value.clone());
    }
  }
  for (source_key, target_key) in [
    ("pluginRunnerConnectorId", "connectorId"),
    ("pluginRunnerConnectorIds", "connectorIds"),
    ("pluginRunnerConnectorServices", "connectorServices"),
  ] {
    if let Some(value) = observation_attributes.get(source_key) {
      attributes.insert(target_key.to_string(), value.clone());
    }
  }
}

#[cfg(test)]
mod tests {
  use pith_plugin_host::PluginCommandEntry as HostPluginCommandEntry;

  use super::*;

  #[test]
  fn approved_plugin_handoff_summarizes_primary_observation() {
    let output = PluginCommandOutput {
      thread_id: "thread-1".to_string(),
      command: test_command(),
      workspace: None,
      input: Some("Draft a page".to_string()),
      items: vec![
        TimelineItem {
          kind: "pluginCommand".to_string(),
          title: "Prepare Notion Page Draft".to_string(),
          content: "Run connector command.".to_string(),
          attributes: None,
        },
        TimelineItem {
          kind: "pluginResult".to_string(),
          title: "Notion Page Draft".to_string(),
          content: "Prepared a local Notion page draft.".to_string(),
          attributes: Some(HashMap::from([(
            "connectorServices".to_string(),
            "notion".to_string(),
          )])),
        },
      ],
      capture_memory: false,
      runner_memory_notes: vec![],
      pending_approval: None,
    };

    let item = build_approved_plugin_handoff(&output).expect("handoff item");
    let attributes = item.attributes.as_ref().expect("attributes");

    assert_eq!(item.kind, "assistantMessage");
    assert!(item.content.contains("Prepare Notion Page Draft completed"));
    assert!(item.content.contains("Notion Page Draft"));
    assert_eq!(
      attributes.get("pluginCommandHandoff").map(String::as_str),
      Some("approvedPluginCommand")
    );
    assert_eq!(
      attributes.get("connectorServices").map(String::as_str),
      Some("notion")
    );
  }

  #[test]
  fn approved_plugin_handoff_skips_outputs_without_observations() {
    let output = PluginCommandOutput {
      thread_id: "thread-1".to_string(),
      command: test_command(),
      workspace: None,
      input: None,
      items: vec![TimelineItem {
        kind: "pluginCommand".to_string(),
        title: "Prepare Notion Page Draft".to_string(),
        content: "Run connector command.".to_string(),
        attributes: None,
      }],
      capture_memory: false,
      runner_memory_notes: vec![],
      pending_approval: None,
    };

    assert!(build_approved_plugin_handoff(&output).is_none());
  }

  fn test_command() -> HostPluginCommandEntry {
    HostPluginCommandEntry {
      command_id: "notion-connector::notion.prepare-page-draft".to_string(),
      title: "Prepare Notion Page Draft".to_string(),
      description: "Prepare a local Notion page draft.".to_string(),
      prompt: "Prepare a page draft.".to_string(),
      plugin_id: "notion-connector".to_string(),
      plugin_display_name: "Notion Connector".to_string(),
      permissions: vec![],
      source_path: "plugins/notion/commands/notion.prepare-page-draft.json".to_string(),
      execution: None,
      execution_kind: Some("mcp.notion.preparePageDraft".to_string()),
      manifest_error: None,
      memory_note_title: None,
      memory_note_source: None,
      memory_note_tags: vec![],
    }
  }
}
