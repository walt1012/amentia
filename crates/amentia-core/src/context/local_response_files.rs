use std::collections::HashMap;

use amentia_memory::MemoryNote;
use amentia_model_runtime::{GenerationCancellation, LocalModelRuntime};
use amentia_tools::{DirectoryEntry, ReadFileResult};

use super::local_response_formatting::format_directory_result;
use super::local_response_generation::{generate_local_summary, mark_cowork_handoff};
use crate::context_memory_pack::{format_memory_context_prompt, pack_memory_notes_for_context};
use crate::context_observation::{
  compact_prompt_observation, merge_prior_observation_attributes, PriorObservationContext,
};

pub(crate) fn summarize_file_result(
  model_runtime: &LocalModelRuntime,
  memory_notes: &[MemoryNote],
  memory_query: &str,
  thread_title: &str,
  workspace_name: &str,
  result: &ReadFileResult,
  prior_observations: Option<&PriorObservationContext>,
  cancellation: Option<&GenerationCancellation>,
) -> (String, HashMap<String, String>) {
  let memory_context = pack_memory_notes_for_context(
    model_runtime,
    memory_notes,
    Some(workspace_name),
    &format!("{memory_query} {}", result.relative_path),
  );
  let preview = result
    .content
    .lines()
    .find(|line| !line.trim().is_empty())
    .unwrap_or("The file is empty.");

  let prior_observation_count = prior_observations
    .map(|context| context.observation_count)
    .unwrap_or_default();
  let observation_summary = if prior_observation_count > 0 {
    format!(
      "Amentia inspected {} for {} in {} after {} prior observation(s). First useful line: {}",
      result.relative_path, thread_title, workspace_name, prior_observation_count, preview
    )
  } else {
    format!(
      "Amentia inspected {} for {} in {}. First useful line: {}",
      result.relative_path, thread_title, workspace_name, preview
    )
  };
  let observation = compact_prompt_observation(&result.content, &memory_context);
  let prior_observation_prompt = prior_observations
    .filter(|context| context.observation_count > 0)
    .map(|context| format!("Prior observations:\n{}\n", context.text))
    .unwrap_or_default();
  let prompt = format!(
    "You are Amentia, a concise local cowork agent. Summarize a file inspection in one or two sentences.\nThread: {thread_title}\nWorkspace: {workspace_name}\n{}\n{}File: {}\nPreview:\n{}",
    format_memory_context_prompt(&memory_context),
    prior_observation_prompt,
    result.relative_path,
    observation.text
  );

  let (summary, mut attributes) = generate_local_summary(
    model_runtime,
    prompt,
    observation_summary,
    &memory_context,
    Some(&observation),
    cancellation,
  );
  if let Some(prior_observations) = prior_observations {
    merge_prior_observation_attributes(&mut attributes, prior_observations);
  }
  mark_cowork_handoff(&mut attributes, "workspaceFile");

  (summary, attributes)
}

pub(crate) fn summarize_directory_result(
  model_runtime: &LocalModelRuntime,
  memory_notes: &[MemoryNote],
  memory_query: &str,
  thread_title: &str,
  workspace_name: &str,
  entries: &[DirectoryEntry],
  cancellation: Option<&GenerationCancellation>,
) -> (String, HashMap<String, String>) {
  let memory_context = pack_memory_notes_for_context(
    model_runtime,
    memory_notes,
    Some(workspace_name),
    &format!("{memory_query} workspace root"),
  );
  if entries.is_empty() {
    let (summary, mut attributes) = generate_local_summary(
      model_runtime,
      format!(
        "You are Amentia, a concise local cowork agent. Summarize an empty workspace root inspection.\nThread: {thread_title}\nWorkspace: {workspace_name}\n{}",
        format_memory_context_prompt(&memory_context)
      ),
      format!(
        "Amentia inspected {} for {} and found an empty root directory.",
        workspace_name, thread_title
      ),
      &memory_context,
      None,
      cancellation,
    );
    mark_cowork_handoff(&mut attributes, "workspaceDirectory");
    return (summary, attributes);
  }

  let preview = entries
    .iter()
    .take(5)
    .map(|entry| entry.name.clone())
    .collect::<Vec<_>>()
    .join(", ");

  let observation_summary = format!(
    "Amentia inspected {} for {} and found {} root entries, including {}.",
    workspace_name,
    thread_title,
    entries.len(),
    preview
  );
  let observation = compact_prompt_observation(&format_directory_result(entries), &memory_context);
  let prompt = format!(
    "You are Amentia, a concise local cowork agent. Summarize a root directory inspection in one or two sentences.\nThread: {thread_title}\nWorkspace: {workspace_name}\n{}\nEntries:\n{}",
    format_memory_context_prompt(&memory_context),
    observation.text
  );

  let (summary, mut attributes) = generate_local_summary(
    model_runtime,
    prompt,
    observation_summary,
    &memory_context,
    Some(&observation),
    cancellation,
  );
  mark_cowork_handoff(&mut attributes, "workspaceDirectory");
  (summary, attributes)
}
