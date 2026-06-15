use super::plugin_command_runner_contracts::{
  PluginRunnerMemoryNoteEnvelope, PLUGIN_RUNNER_MEMORY_NOTE_BODY_LIMIT,
  PLUGIN_RUNNER_MEMORY_NOTE_LIMIT, PLUGIN_RUNNER_MEMORY_NOTE_TAG_LENGTH_LIMIT,
  PLUGIN_RUNNER_MEMORY_NOTE_TAG_LIMIT, PLUGIN_RUNNER_MEMORY_NOTE_TITLE_LIMIT,
};
use super::plugin_command_types::PluginRunnerMemoryNoteDraft;

pub(super) struct PluginRunnerMemoryNoteSelection {
  pub(super) notes: Vec<PluginRunnerMemoryNoteDraft>,
  pub(super) invalid_count: usize,
  pub(super) truncated_count: usize,
}

pub(super) fn plugin_runner_memory_notes(
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
