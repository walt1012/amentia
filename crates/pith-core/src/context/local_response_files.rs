use std::collections::HashMap;

use pith_memory::MemoryNote;
use pith_model_runtime::{GenerationCancellation, LocalModelRuntime};
use pith_tools::{DirectoryEntry, ReadFileResult};

use super::local_response_formatting::format_directory_result;
use super::local_response_generation::generate_local_summary;
use crate::context_compaction::compact_prompt_observation;
use crate::context_memory_pack::{format_memory_context_prompt, pack_memory_notes_for_context};

pub(crate) fn summarize_file_result(
  model_runtime: &LocalModelRuntime,
  memory_notes: &[MemoryNote],
  thread_title: &str,
  workspace_name: &str,
  result: &ReadFileResult,
  cancellation: Option<&GenerationCancellation>,
) -> (String, HashMap<String, String>) {
  let memory_context = pack_memory_notes_for_context(
    model_runtime,
    memory_notes,
    Some(workspace_name),
    &format!("{thread_title} {}", result.relative_path),
  );
  let preview = result
    .content
    .lines()
    .find(|line| !line.trim().is_empty())
    .unwrap_or("The file is empty.");

  let observation_summary = format!(
    "Pith inspected {} for {} in {}. First useful line: {}",
    result.relative_path, thread_title, workspace_name, preview
  );
  let observation = compact_prompt_observation(&result.content, &memory_context);
  let prompt = format!(
    "You are Pith, a concise local coding agent. Summarize a file inspection in one or two sentences.\nThread: {thread_title}\nWorkspace: {workspace_name}\n{}\nFile: {}\nPreview:\n{}",
    format_memory_context_prompt(&memory_context),
    result.relative_path,
    observation.text
  );

  generate_local_summary(
    model_runtime,
    prompt,
    observation_summary,
    &memory_context,
    Some(&observation),
    cancellation,
  )
}

pub(crate) fn summarize_directory_result(
  model_runtime: &LocalModelRuntime,
  memory_notes: &[MemoryNote],
  thread_title: &str,
  workspace_name: &str,
  entries: &[DirectoryEntry],
  cancellation: Option<&GenerationCancellation>,
) -> (String, HashMap<String, String>) {
  let memory_context = pack_memory_notes_for_context(
    model_runtime,
    memory_notes,
    Some(workspace_name),
    &format!("{thread_title} workspace root"),
  );
  if entries.is_empty() {
    return generate_local_summary(
      model_runtime,
      format!(
        "You are Pith, a concise local coding agent. Summarize an empty workspace root inspection.\nThread: {thread_title}\nWorkspace: {workspace_name}\n{}",
        format_memory_context_prompt(&memory_context)
      ),
      format!(
        "Pith inspected {} for {} and found an empty root directory.",
        workspace_name, thread_title
      ),
      &memory_context,
      None,
      cancellation,
    );
  }

  let preview = entries
    .iter()
    .take(5)
    .map(|entry| entry.name.clone())
    .collect::<Vec<_>>()
    .join(", ");

  let observation_summary = format!(
    "Pith inspected {} for {} and found {} root entries, including {}.",
    workspace_name,
    thread_title,
    entries.len(),
    preview
  );
  let observation = compact_prompt_observation(&format_directory_result(entries), &memory_context);
  let prompt = format!(
    "You are Pith, a concise local coding agent. Summarize a root directory inspection in one or two sentences.\nThread: {thread_title}\nWorkspace: {workspace_name}\n{}\nEntries:\n{}",
    format_memory_context_prompt(&memory_context),
    observation.text
  );

  generate_local_summary(
    model_runtime,
    prompt,
    observation_summary,
    &memory_context,
    Some(&observation),
    cancellation,
  )
}
