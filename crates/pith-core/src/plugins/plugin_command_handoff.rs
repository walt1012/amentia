use std::collections::HashMap;

use pith_protocol::TimelineItem;

use super::plugin_command_types::PluginCommandOutput;

const HANDOFF_PREVIEW_LIMIT: usize = 360;
const OBSERVATION_HANDOFF_KEYS: &[&str] = &[
  "connectorId",
  "connectorIds",
  "connectorServices",
  "targetService",
  "targetTool",
  "draftMode",
  "remoteWrite",
  "remoteWriteStage",
  "remoteWriteRequiresApproval",
  "sourceArtifact",
  "sourceArtifactPreviewProvided",
];
const RUNNER_CONNECTOR_HANDOFF_KEYS: &[(&str, &str)] = &[
  ("pluginRunnerConnectorId", "connectorId"),
  ("pluginRunnerConnectorIds", "connectorIds"),
  ("pluginRunnerConnectorServices", "connectorServices"),
];

pub(crate) fn ensure_plugin_command_handoff(
  output: &mut PluginCommandOutput,
  handoff_kind: &'static str,
) {
  let Some(observation) = primary_observation_summary(&output.items) else {
    return;
  };
  let attributes = plugin_handoff_attributes(output, handoff_kind, &observation);
  let target_item = last_command_assistant_item_mut(&mut output.items, &output.command.command_id);
  if let Some(item) = target_item {
    item
      .attributes
      .get_or_insert_with(HashMap::new)
      .extend(attributes);
    return;
  }

  output.items.push(TimelineItem {
    kind: "assistantMessage".to_string(),
    title: "Assistant".to_string(),
    content: format!(
      "{} completed. Observation: {}. {}",
      output.command.title, observation.title, observation.preview
    ),
    attributes: Some(attributes),
  });
}

#[derive(Debug, Clone)]
struct PluginObservationSummary {
  kind: String,
  title: String,
  preview: String,
  attributes: Option<HashMap<String, String>>,
}

fn primary_observation_summary(items: &[TimelineItem]) -> Option<PluginObservationSummary> {
  items
    .iter()
    .rev()
    .find(|item| item.kind == "pluginResult" || item.kind == "toolResult")
    .map(|item| PluginObservationSummary {
      kind: item.kind.clone(),
      title: item.title.trim().to_string(),
      preview: bounded_preview(&item.content),
      attributes: item.attributes.clone(),
    })
}

fn plugin_handoff_attributes(
  output: &PluginCommandOutput,
  handoff_kind: &'static str,
  observation: &PluginObservationSummary,
) -> HashMap<String, String> {
  let mut attributes = HashMap::from([
    ("pluginId".to_string(), output.command.plugin_id.clone()),
    ("commandId".to_string(), output.command.command_id.clone()),
    ("pluginCommandHandoff".to_string(), handoff_kind.to_string()),
    (
      "pluginCommandObservationKind".to_string(),
      observation.kind.clone(),
    ),
    (
      "pluginCommandObservationTitle".to_string(),
      observation.title.clone(),
    ),
  ]);
  if let Some(execution_kind) = output.command.execution_kind.as_ref() {
    attributes.insert("executionKind".to_string(), execution_kind.clone());
  }
  copy_observation_handoff_attributes(&mut attributes, observation.attributes.as_ref());
  attributes
}

fn last_command_assistant_item_mut<'a>(
  items: &'a mut [TimelineItem],
  command_id: &str,
) -> Option<&'a mut TimelineItem> {
  items.iter_mut().rev().find(|item| {
    item.kind == "assistantMessage"
      && item
        .attributes
        .as_ref()
        .and_then(|attributes| attributes.get("commandId"))
        .map(|value| value == command_id)
        .unwrap_or(false)
  })
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

fn copy_observation_handoff_attributes(
  attributes: &mut HashMap<String, String>,
  observation_attributes: Option<&HashMap<String, String>>,
) {
  let Some(observation_attributes) = observation_attributes else {
    return;
  };
  for key in OBSERVATION_HANDOFF_KEYS.iter().copied() {
    if let Some(value) = observation_attributes.get(key) {
      attributes.insert(key.to_string(), value.clone());
    }
  }
  for (source_key, target_key) in RUNNER_CONNECTOR_HANDOFF_KEYS.iter().copied() {
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
  fn plugin_handoff_summarizes_primary_observation() {
    let mut output = PluginCommandOutput {
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

    ensure_plugin_command_handoff(&mut output, "approvedPluginCommand");
    let item = output.items.last().expect("handoff item");
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
  fn plugin_handoff_marks_existing_assistant_item() {
    let mut output = PluginCommandOutput {
      thread_id: "thread-1".to_string(),
      command: test_command(),
      workspace: None,
      input: None,
      items: vec![
        TimelineItem {
          kind: "pluginResult".to_string(),
          title: "Notion Page Draft".to_string(),
          content: "Prepared a local Notion page draft.".to_string(),
          attributes: None,
        },
        TimelineItem {
          kind: "assistantMessage".to_string(),
          title: "Assistant".to_string(),
          content: "Prepare Notion Page Draft completed.".to_string(),
          attributes: Some(HashMap::from([(
            "commandId".to_string(),
            "notion-connector::notion.prepare-page-draft".to_string(),
          )])),
        },
      ],
      capture_memory: false,
      runner_memory_notes: vec![],
      pending_approval: None,
    };

    ensure_plugin_command_handoff(&mut output, "pluginCommand");

    assert_eq!(output.items.len(), 2);
    let attributes = output.items[1].attributes.as_ref().expect("attributes");
    assert_eq!(
      attributes.get("pluginCommandHandoff").map(String::as_str),
      Some("pluginCommand")
    );
  }

  #[test]
  fn plugin_handoff_preserves_remote_write_inspection_metadata() {
    let mut output = PluginCommandOutput {
      thread_id: "thread-1".to_string(),
      command: test_command(),
      workspace: None,
      input: Some("Publish docs/handoff.md".to_string()),
      items: vec![TimelineItem {
        kind: "pluginResult".to_string(),
        title: "Notion Remote Write Inspection".to_string(),
        content: "No remote write was sent.".to_string(),
        attributes: Some(HashMap::from([
          ("targetService".to_string(), "notion".to_string()),
          (
            "targetTool".to_string(),
            "notion.inspectPageWrite".to_string(),
          ),
          ("remoteWrite".to_string(), "false".to_string()),
          (
            "remoteWriteStage".to_string(),
            "inspectBeforeWrite".to_string(),
          ),
          (
            "remoteWriteRequiresApproval".to_string(),
            "true".to_string(),
          ),
          ("sourceArtifact".to_string(), "docs/handoff.md".to_string()),
        ])),
      }],
      capture_memory: false,
      runner_memory_notes: vec![],
      pending_approval: None,
    };

    ensure_plugin_command_handoff(&mut output, "approvedPluginCommand");
    let attributes = output
      .items
      .last()
      .and_then(|item| item.attributes.as_ref())
      .expect("handoff attributes");

    assert_eq!(
      attributes.get("remoteWriteStage").map(String::as_str),
      Some("inspectBeforeWrite")
    );
    assert_eq!(
      attributes
        .get("remoteWriteRequiresApproval")
        .map(String::as_str),
      Some("true")
    );
    assert_eq!(
      attributes.get("sourceArtifact").map(String::as_str),
      Some("docs/handoff.md")
    );
  }

  #[test]
  fn plugin_handoff_skips_outputs_without_observations() {
    let mut output = PluginCommandOutput {
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

    ensure_plugin_command_handoff(&mut output, "pluginCommand");

    assert_eq!(output.items.len(), 1);
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
