use std::collections::HashMap;

use pith_plugin_host::PluginCommandEntry as HostPluginCommandEntry;
use pith_protocol::TimelineItem;
use serde::Deserialize;

use super::plugin_command_runner::{
  PluginRunnerFailure, PluginRunnerResult, PluginRunnerRunResult,
};
use super::plugin_command_types::PluginRunnerMemoryNoteDraft;

const PLUGIN_RUNNER_LOG_PREVIEW_LIMIT: usize = 2048;
const PLUGIN_RUNNER_MEMORY_NOTE_LIMIT: usize = 4;
const PLUGIN_RUNNER_MEMORY_NOTE_TITLE_LIMIT: usize = 120;
const PLUGIN_RUNNER_MEMORY_NOTE_BODY_LIMIT: usize = 4096;
const PLUGIN_RUNNER_MEMORY_NOTE_TAG_LIMIT: usize = 8;
const PLUGIN_RUNNER_MEMORY_NOTE_TAG_LENGTH_LIMIT: usize = 40;
const PLUGIN_RUNNER_ALLOWED_TIMELINE_KINDS: &[&str] =
  &["pluginResult", "toolResult", "warning", "system"];
const PLUGIN_RUNNER_REMOTE_WRITE_CONTRACT: &str = "pith.connectorRemoteWrite.v1";
const PLUGIN_RUNNER_REMOTE_WRITE_COMPLETED_STAGE: &str = "completed";

struct PluginRunnerMemoryNoteSelection {
  notes: Vec<PluginRunnerMemoryNoteDraft>,
  invalid_count: usize,
  truncated_count: usize,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PluginRunnerOutputEnvelope {
  content: Option<String>,
  message: Option<String>,
  #[serde(default)]
  items: Vec<PluginRunnerTimelineItemEnvelope>,
  #[serde(default)]
  memory_notes: Vec<PluginRunnerMemoryNoteEnvelope>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PluginRunnerTimelineItemEnvelope {
  kind: String,
  title: String,
  content: String,
  #[serde(default)]
  attributes: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PluginRunnerMemoryNoteEnvelope {
  title: Option<String>,
  body: Option<String>,
  source: Option<String>,
  #[serde(default)]
  tags: Vec<String>,
}

pub(super) fn plugin_runner_output(
  command: &HostPluginCommandEntry,
  execution_kind: &str,
  output: &str,
  mut attributes: HashMap<String, String>,
) -> PluginRunnerRunResult<PluginRunnerResult> {
  let envelope = match serde_json::from_str::<PluginRunnerOutputEnvelope>(output) {
    Ok(envelope) => envelope,
    Err(error) => {
      if plugin_runner_output_looks_like_json(output) {
        attributes.insert(
          "pluginRunnerOutputStatus".to_string(),
          "malformedEnvelope".to_string(),
        );
        attributes.insert("pluginRunnerOutputParsed".to_string(), "false".to_string());
        attributes.insert(
          "pluginRunnerOutputParseError".to_string(),
          bounded_log_preview(&error.to_string()),
        );
        return Err(
          PluginRunnerFailure::with_output(
            -32054,
            format!(
              "Plugin command `{}` returned a malformed JSON output envelope: {error}",
              command.command_id
            ),
            output.to_string(),
            String::new(),
            attributes,
          )
          .boxed(),
        );
      }

      attributes.insert(
        "pluginRunnerOutputStatus".to_string(),
        "plainText".to_string(),
      );
      attributes.insert("pluginRunnerOutputParsed".to_string(), "false".to_string());
      return Ok(PluginRunnerResult {
        execution_kind: execution_kind.to_string(),
        content: plugin_runner_content(output),
        items: vec![],
        memory_notes: vec![],
        attributes,
      });
    }
  };
  let content = envelope
    .content
    .or(envelope.message)
    .map(|content| content.trim().to_string())
    .filter(|content| !content.is_empty())
    .unwrap_or_default();
  let (items, invalid_item_count) =
    plugin_runner_timeline_items(command, execution_kind, &attributes, envelope.items);
  let memory_note_selection = plugin_runner_memory_notes(envelope.memory_notes);
  insert_plugin_runner_output_attributes(
    &mut attributes,
    &content,
    items.len(),
    invalid_item_count,
    memory_note_selection.notes.len(),
    memory_note_selection.invalid_count,
    memory_note_selection.truncated_count,
  );
  if invalid_item_count > 0 || memory_note_selection.invalid_count > 0 {
    attributes.insert(
      "pluginRunnerOutputStatus".to_string(),
      "invalidEnvelope".to_string(),
    );
    return Err(
      PluginRunnerFailure::with_output(
        -32054,
        format!(
          "Plugin command `{}` returned an output envelope with invalid timeline items or memory notes.",
          command.command_id
        ),
        output.to_string(),
        String::new(),
        attributes,
      )
      .boxed(),
    );
  }
  let memory_notes = memory_note_selection.notes;
  if content.is_empty() && items.is_empty() && memory_notes.is_empty() {
    attributes.insert(
      "pluginRunnerOutputStatus".to_string(),
      "emptyEnvelope".to_string(),
    );
    return Err(
      PluginRunnerFailure::with_output(
        -32054,
        format!(
          "Plugin command `{}` returned an output envelope without content, valid timeline items, or memory notes.",
          command.command_id
        ),
        output.to_string(),
        String::new(),
        attributes,
      )
      .boxed(),
    );
  }
  attributes.insert(
    "pluginRunnerOutputStatus".to_string(),
    "envelope".to_string(),
  );
  let items = plugin_runner_timeline_items_with_attributes(items, &attributes);

  Ok(PluginRunnerResult {
    execution_kind: execution_kind.to_string(),
    content: if content.is_empty() {
      if memory_notes.is_empty() {
        "Plugin command completed with timeline items.".to_string()
      } else {
        "Plugin command completed with memory notes.".to_string()
      }
    } else {
      content
    },
    items,
    memory_notes,
    attributes,
  })
}

pub(super) fn bounded_log_preview(content: &str) -> String {
  let mut preview = content
    .chars()
    .take(PLUGIN_RUNNER_LOG_PREVIEW_LIMIT)
    .collect::<String>();
  if content.chars().count() > PLUGIN_RUNNER_LOG_PREVIEW_LIMIT {
    preview.push_str("\n[truncated]");
  }
  preview
}

fn plugin_runner_timeline_items(
  command: &HostPluginCommandEntry,
  execution_kind: &str,
  base_attributes: &HashMap<String, String>,
  items: Vec<PluginRunnerTimelineItemEnvelope>,
) -> (Vec<TimelineItem>, usize) {
  let total_item_count = items.len();
  let valid_items = items
    .into_iter()
    .filter_map(|item| plugin_runner_timeline_item(command, execution_kind, base_attributes, item))
    .collect::<Vec<_>>();
  let invalid_item_count = total_item_count.saturating_sub(valid_items.len());

  (valid_items, invalid_item_count)
}

fn plugin_runner_timeline_items_with_attributes(
  items: Vec<TimelineItem>,
  attributes: &HashMap<String, String>,
) -> Vec<TimelineItem> {
  items
    .into_iter()
    .map(|mut item| {
      let item_attributes = item.attributes.get_or_insert_with(HashMap::new);
      for (key, value) in attributes {
        if plugin_runner_reserved_attribute(key) {
          item_attributes.insert(key.clone(), value.clone());
        } else {
          item_attributes
            .entry(key.clone())
            .or_insert_with(|| value.clone());
        }
      }
      item
    })
    .collect()
}

fn insert_plugin_runner_output_attributes(
  attributes: &mut HashMap<String, String>,
  content: &str,
  valid_item_count: usize,
  invalid_item_count: usize,
  memory_note_count: usize,
  invalid_memory_note_count: usize,
  truncated_memory_note_count: usize,
) {
  attributes.insert("pluginRunnerOutputParsed".to_string(), "true".to_string());
  attributes.insert(
    "pluginRunnerOutputContentBytes".to_string(),
    content.len().to_string(),
  );
  attributes.insert(
    "pluginRunnerOutputValidTimelineItemCount".to_string(),
    valid_item_count.to_string(),
  );
  attributes.insert(
    "pluginRunnerOutputInvalidTimelineItemCount".to_string(),
    invalid_item_count.to_string(),
  );
  attributes.insert(
    "pluginRunnerOutputMemoryNoteCount".to_string(),
    memory_note_count.to_string(),
  );
  attributes.insert(
    "pluginRunnerOutputInvalidMemoryNoteCount".to_string(),
    invalid_memory_note_count.to_string(),
  );
  attributes.insert(
    "pluginRunnerOutputTruncatedMemoryNoteCount".to_string(),
    truncated_memory_note_count.to_string(),
  );
}

fn plugin_runner_memory_notes(
  notes: Vec<PluginRunnerMemoryNoteEnvelope>,
) -> PluginRunnerMemoryNoteSelection {
  let mut selected_notes = vec![];
  let mut invalid_count = 0;
  let mut truncated_count = 0;

  for note in notes {
    let Some(note) = plugin_runner_memory_note(note) else {
      invalid_count += 1;
      continue;
    };
    if selected_notes.len() < PLUGIN_RUNNER_MEMORY_NOTE_LIMIT {
      selected_notes.push(note);
    } else {
      truncated_count += 1;
    }
  }

  PluginRunnerMemoryNoteSelection {
    notes: selected_notes,
    invalid_count,
    truncated_count,
  }
}

fn plugin_runner_memory_note(
  note: PluginRunnerMemoryNoteEnvelope,
) -> Option<PluginRunnerMemoryNoteDraft> {
  let title = note.title.as_deref().map(str::trim).unwrap_or_default();
  let body = note.body.as_deref().map(str::trim).unwrap_or_default();
  if title.is_empty() || body.is_empty() {
    return None;
  }

  Some(PluginRunnerMemoryNoteDraft {
    title: bounded_runner_memory_text(title, PLUGIN_RUNNER_MEMORY_NOTE_TITLE_LIMIT),
    body: bounded_runner_memory_text(body, PLUGIN_RUNNER_MEMORY_NOTE_BODY_LIMIT),
    source: note
      .source
      .as_deref()
      .map(str::trim)
      .filter(|source| !source.is_empty())
      .map(str::to_string),
    tags: note
      .tags
      .into_iter()
      .take(PLUGIN_RUNNER_MEMORY_NOTE_TAG_LIMIT)
      .map(|tag| tag.trim().to_string())
      .filter(|tag| !tag.is_empty())
      .map(|tag| bounded_runner_memory_text(&tag, PLUGIN_RUNNER_MEMORY_NOTE_TAG_LENGTH_LIMIT))
      .collect(),
  })
}

fn bounded_runner_memory_text(value: &str, limit: usize) -> String {
  value.chars().take(limit).collect()
}

fn plugin_runner_content(output: &str) -> String {
  if output.trim().is_empty() {
    return "Plugin command completed without output.".to_string();
  }

  output.to_string()
}

fn plugin_runner_output_looks_like_json(output: &str) -> bool {
  let trimmed = output.trim_start();
  trimmed.starts_with('{') || trimmed.starts_with('[')
}

fn plugin_runner_timeline_item(
  command: &HostPluginCommandEntry,
  execution_kind: &str,
  base_attributes: &HashMap<String, String>,
  item: PluginRunnerTimelineItemEnvelope,
) -> Option<TimelineItem> {
  let kind = item.kind.trim();
  let title = item.title.trim();
  let content = item.content.trim();
  if !plugin_runner_timeline_kind_is_allowed(kind) || title.is_empty() || content.is_empty() {
    return None;
  }

  let mut attributes = plugin_runner_owned_attributes(item.attributes);
  attributes.extend(base_attributes.clone());
  attributes
    .entry("pluginId".to_string())
    .or_insert_with(|| command.plugin_id.clone());
  attributes
    .entry("commandId".to_string())
    .or_insert_with(|| command.command_id.clone());
  attributes
    .entry("executionKind".to_string())
    .or_insert_with(|| execution_kind.to_string());
  attributes
    .entry("sourcePath".to_string())
    .or_insert_with(|| command.source_path.clone());

  if !plugin_runner_remote_write_contract_is_valid(&attributes) {
    return None;
  }
  insert_plugin_runner_remote_write_contract(&mut attributes);

  Some(TimelineItem {
    kind: kind.to_string(),
    title: title.to_string(),
    content: content.to_string(),
    attributes: Some(attributes),
  })
}

fn plugin_runner_timeline_kind_is_allowed(kind: &str) -> bool {
  PLUGIN_RUNNER_ALLOWED_TIMELINE_KINDS.contains(&kind)
}

fn plugin_runner_remote_write_contract_is_valid(attributes: &HashMap<String, String>) -> bool {
  let remote_write = attributes
    .get("remoteWrite")
    .map(|value| value.trim())
    .unwrap_or_default();

  match remote_write {
    "" | "false" => true,
    "true" => {
      plugin_runner_attribute_value(attributes, "remoteWriteStage")
        == Some(PLUGIN_RUNNER_REMOTE_WRITE_COMPLETED_STAGE)
        && plugin_runner_attribute_value(attributes, "targetService").is_some()
        && plugin_runner_attribute_value(attributes, "targetTool").is_some()
    }
    _ => false,
  }
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
  }
}

fn plugin_runner_owned_attributes(attributes: HashMap<String, String>) -> HashMap<String, String> {
  attributes
    .into_iter()
    .filter(|(key, _)| !plugin_runner_reserved_attribute(key))
    .collect()
}

fn plugin_runner_reserved_attribute(key: &str) -> bool {
  matches!(
    key,
    "action"
      | "approvalId"
      | "commandId"
      | "commandInput"
      | "connectorId"
      | "connectorIds"
      | "connectorRepairHint"
      | "connectorStatus"
      | "decision"
      | "executionKind"
      | "memoryNoteId"
      | "memoryNoteTitle"
      | "memoryScope"
      | "permissionGate"
      | "pluginCommandRunId"
      | "pluginCommandStatus"
      | "pluginDisplayName"
      | "pluginId"
      | "pluginInstallStatus"
      | "pluginLifecycleStatus"
      | "pluginSourcePath"
      | "requiredPermission"
      | "runBlocker"
      | "runRepairHint"
      | "runStatus"
      | "sourcePath"
      | "streamingStatus"
      | "turnId"
  ) || key.starts_with("connector")
    || key.starts_with("mcp")
    || key.starts_with("pluginRunner")
    || key.starts_with("sandbox")
}

#[cfg(test)]
mod tests {
  use std::collections::HashMap;

  use pith_plugin_host::PluginCommandEntry as HostPluginCommandEntry;

  use super::plugin_runner_output;

  #[test]
  fn output_contract_rejects_invalid_memory_notes_instead_of_partial_success() {
    let command = test_command();
    let output = r#"{
      "content": "Partial content should not mask invalid memory.",
      "memoryNotes": [
        { "title": "Missing Body" }
      ]
    }"#;

    let failure = match plugin_runner_output(&command, "stdio.test", output, HashMap::new()) {
      Ok(_) => panic!("invalid output envelope should fail"),
      Err(failure) => failure,
    };

    assert_eq!(failure.code, -32054);
    assert_eq!(
      failure
        .attributes
        .get("pluginRunnerOutputStatus")
        .map(String::as_str),
      Some("invalidEnvelope")
    );
    assert_eq!(
      failure
        .attributes
        .get("pluginRunnerOutputInvalidMemoryNoteCount")
        .map(String::as_str),
      Some("1")
    );
    assert!(failure
      .message
      .contains("invalid timeline items or memory notes"));
  }

  #[test]
  fn output_contract_truncates_extra_valid_memory_notes_without_failing() {
    let command = test_command();
    let output = r#"{
      "content": "Memory notes captured.",
      "memoryNotes": [
        { "title": "Note 1", "body": "Body 1" },
        { "title": "Note 2", "body": "Body 2" },
        { "title": "Note 3", "body": "Body 3" },
        { "title": "Note 4", "body": "Body 4" },
        { "title": "Note 5", "body": "Body 5" }
      ]
    }"#;

    let result = match plugin_runner_output(&command, "stdio.test", output, HashMap::new()) {
      Ok(result) => result,
      Err(failure) => panic!(
        "valid oversized memory note list failed: {}",
        failure.message
      ),
    };

    assert_eq!(result.memory_notes.len(), 4);
    assert_eq!(
      result
        .attributes
        .get("pluginRunnerOutputInvalidMemoryNoteCount")
        .map(String::as_str),
      Some("0")
    );
    assert_eq!(
      result
        .attributes
        .get("pluginRunnerOutputTruncatedMemoryNoteCount")
        .map(String::as_str),
      Some("1")
    );
  }

  #[test]
  fn output_contract_protects_core_timeline_metadata() {
    let command = test_command();
    let mut base_attributes = HashMap::new();
    base_attributes.insert("sandboxMode".to_string(), "workspaceReadWrite".to_string());
    base_attributes.insert(
      "pluginRunnerOutputStatus".to_string(),
      "baseStatus".to_string(),
    );
    let output = r#"{
      "items": [
        {
          "kind": "pluginResult",
          "title": "Runner Item",
          "content": "Owned timeline item.",
          "attributes": {
            "runner": "stdio",
            "pluginId": "spoofed-plugin",
            "commandId": "spoofed-command",
            "approvalId": "approval-1",
            "turnId": "turn-1",
            "sandboxMode": "none",
            "pluginRunnerOutputStatus": "spoofed"
          }
        }
      ]
    }"#;

    let result = match plugin_runner_output(&command, "stdio.test", output, base_attributes) {
      Ok(result) => result,
      Err(failure) => panic!(
        "trusted output metadata should be protected: {}",
        failure.message
      ),
    };
    let attributes = result.items[0].attributes.as_ref().expect("attributes");

    assert_eq!(attributes.get("runner").map(String::as_str), Some("stdio"));
    assert_eq!(
      attributes.get("pluginId").map(String::as_str),
      Some("test-plugin")
    );
    assert_eq!(
      attributes.get("commandId").map(String::as_str),
      Some("test-plugin::run")
    );
    assert_eq!(
      attributes.get("sandboxMode").map(String::as_str),
      Some("workspaceReadWrite")
    );
    assert_eq!(
      attributes
        .get("pluginRunnerOutputStatus")
        .map(String::as_str),
      Some("envelope")
    );
    assert!(!attributes.contains_key("approvalId"));
    assert!(!attributes.contains_key("turnId"));
  }

  #[test]
  fn output_contract_rejects_action_timeline_kinds_from_runner() {
    let command = test_command();
    let output = r#"{
      "items": [
        {
          "kind": "approvalRequested",
          "title": "Fake Approval",
          "content": "This must not become a trusted action."
        }
      ]
    }"#;

    let failure = match plugin_runner_output(&command, "stdio.test", output, HashMap::new()) {
      Ok(_) => panic!("untrusted action timeline item should fail"),
      Err(failure) => failure,
    };

    assert_eq!(failure.code, -32054);
    assert_eq!(
      failure
        .attributes
        .get("pluginRunnerOutputInvalidTimelineItemCount")
        .map(String::as_str),
      Some("1")
    );
  }

  #[test]
  fn output_contract_accepts_remote_write_inspection_items() {
    let command = test_command();
    let output = r#"{
      "items": [
        {
          "kind": "pluginResult",
          "title": "Remote Write Inspection",
          "content": "This is an inspection item and has not written remotely.",
          "attributes": {
            "remoteWrite": "false",
            "remoteWriteStage": "inspectBeforeWrite",
            "targetService": "notion",
            "targetTool": "notion.inspectPageWrite"
          }
        }
      ]
    }"#;

    let result = match plugin_runner_output(&command, "stdio.test", output, HashMap::new()) {
      Ok(result) => result,
      Err(failure) => panic!(
        "inspection item should be accepted: {}",
        failure.message
      ),
    };
    let attributes = result.items[0].attributes.as_ref().expect("attributes");

    assert_eq!(
      attributes
        .get("pluginRunnerRemoteWriteContract")
        .map(String::as_str),
      Some("pith.connectorRemoteWrite.v1")
    );
    assert_eq!(
      attributes.get("remoteWrite").map(String::as_str),
      Some("false")
    );
  }

  #[test]
  fn output_contract_rejects_unproven_remote_write_claims() {
    let command = test_command();
    let output = r#"{
      "items": [
        {
          "kind": "pluginResult",
          "title": "Remote Write Complete",
          "content": "The page was updated.",
          "attributes": {
            "remoteWrite": "true",
            "targetService": "notion"
          }
        }
      ]
    }"#;

    let failure = match plugin_runner_output(&command, "stdio.test", output, HashMap::new()) {
      Ok(_) => panic!("unproven remote write claim should fail"),
      Err(failure) => failure,
    };

    assert_eq!(failure.code, -32054);
    assert_eq!(
      failure
        .attributes
        .get("pluginRunnerOutputInvalidTimelineItemCount")
        .map(String::as_str),
      Some("1")
    );
  }

  #[test]
  fn output_contract_accepts_completed_remote_write_with_target_evidence() {
    let command = test_command();
    let output = r#"{
      "items": [
        {
          "kind": "pluginResult",
          "title": "Remote Write Complete",
          "content": "The page was updated.",
          "attributes": {
            "remoteWrite": "true",
            "remoteWriteStage": "completed",
            "targetService": "notion",
            "targetTool": "notion.updatePage"
          }
        }
      ]
    }"#;

    let result = match plugin_runner_output(&command, "stdio.test", output, HashMap::new()) {
      Ok(result) => result,
      Err(failure) => panic!(
        "completed remote write should be accepted: {}",
        failure.message
      ),
    };
    let attributes = result.items[0].attributes.as_ref().expect("attributes");

    assert_eq!(
      attributes
        .get("pluginRunnerRemoteWriteContract")
        .map(String::as_str),
      Some("pith.connectorRemoteWrite.v1")
    );
    assert_eq!(
      attributes.get("remoteWriteStage").map(String::as_str),
      Some("completed")
    );
  }

  fn test_command() -> HostPluginCommandEntry {
    HostPluginCommandEntry {
      command_id: "test-plugin::run".to_string(),
      title: "Run Test Plugin".to_string(),
      description: "Run a test plugin command.".to_string(),
      prompt: "Run the plugin.".to_string(),
      plugin_id: "test-plugin".to_string(),
      plugin_display_name: "Test Plugin".to_string(),
      permissions: vec![],
      source_path: "plugins/test-plugin/commands/run.json".to_string(),
      execution: None,
      execution_kind: Some("stdio.test".to_string()),
      manifest_error: None,
      memory_note_title: None,
      memory_note_source: None,
      memory_note_tags: vec![],
    }
  }
}
