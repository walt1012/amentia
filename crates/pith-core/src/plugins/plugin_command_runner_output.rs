use std::collections::HashMap;

use pith_plugin_host::PluginCommandEntry as HostPluginCommandEntry;

use super::plugin_command_runner::{
  PluginRunnerFailure, PluginRunnerResult, PluginRunnerRunResult,
};
use super::plugin_command_runner_contracts::PluginRunnerOutputEnvelope;
use super::plugin_command_runner_memory::plugin_runner_memory_notes;
use super::plugin_command_runner_proof::{
  plugin_runner_expected_workflow_id, plugin_runner_items_include_workflow,
};
use super::plugin_command_runner_timeline_output::{
  plugin_runner_timeline_items, plugin_runner_timeline_items_with_attributes,
};

const PLUGIN_RUNNER_LOG_PREVIEW_LIMIT: usize = 2048;

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
  let missing_workflow_item = plugin_runner_expected_workflow_id(command)
    .is_some_and(|workflow_id| !plugin_runner_items_include_workflow(&items, workflow_id));
  insert_plugin_runner_output_attributes(
    &mut attributes,
    &content,
    items.len(),
    invalid_item_count,
    memory_note_selection.notes.len(),
    memory_note_selection.invalid_count,
    memory_note_selection.truncated_count,
  );
  if missing_workflow_item {
    attributes.insert(
      "pluginRunnerOutputStatus".to_string(),
      "missingConnectorWorkflow".to_string(),
    );
    return Err(
      PluginRunnerFailure::with_output(
        -32054,
        format!(
          "Plugin command `{}` is bound to a connector workflow but did not return a valid workflow timeline item.",
          command.command_id
        ),
        output.to_string(),
        String::new(),
        attributes,
      )
      .boxed(),
    );
  }
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

#[cfg(test)]
#[path = "plugin_command_runner_output_tests.rs"]
mod tests;
