use std::collections::HashMap;

use amentia_memory::MemoryNote;
use amentia_model_runtime::{GenerationCancellation, LocalModelRuntime};
use amentia_tools::SearchMatch;

use super::local_response_formatting::format_search_result;
use super::local_response_generation::{generate_local_summary, mark_cowork_handoff};
use crate::context_compaction::compact_prompt_observation;
use crate::context_memory_pack::{format_memory_context_prompt, pack_memory_notes_for_context};

pub(crate) fn summarize_search_result(
  model_runtime: &LocalModelRuntime,
  memory_notes: &[MemoryNote],
  thread_title: &str,
  workspace_name: &str,
  query: &str,
  matches: &[SearchMatch],
  cancellation: Option<&GenerationCancellation>,
) -> (String, HashMap<String, String>) {
  let memory_context =
    pack_memory_notes_for_context(model_runtime, memory_notes, Some(workspace_name), query);
  if matches.is_empty() {
    let (summary, mut attributes) = generate_local_summary(
      model_runtime,
      format!(
        "You are Amentia, a concise local cowork agent. Summarize a search with no matches.\nThread: {thread_title}\nWorkspace: {workspace_name}\n{}\nQuery: {query}",
        format_memory_context_prompt(&memory_context)
      ),
      format!(
        "Amentia searched {} for {} and found no matches for \"{}\".",
        workspace_name, thread_title, query
      ),
      &memory_context,
      None,
      cancellation,
    );
    mark_cowork_handoff(&mut attributes, "workspaceSearchNoMatches");
    return (summary, attributes);
  }

  let preview = matches
    .iter()
    .take(3)
    .map(|entry| format!("{}:{}", entry.relative_path, entry.line_number))
    .collect::<Vec<_>>()
    .join(", ");

  let observation_summary = format!(
    "Amentia searched {} for {} and found {} matches for \"{}\", including {}.",
    workspace_name,
    thread_title,
    matches.len(),
    query,
    preview
  );
  let observation =
    compact_prompt_observation(&format_search_result(query, matches), &memory_context);
  let prompt = format!(
    "You are Amentia, a concise local cowork agent. Summarize a workspace search in one or two sentences.\nThread: {thread_title}\nWorkspace: {workspace_name}\n{}\nQuery: {query}\nMatches:\n{}",
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
  mark_cowork_handoff(&mut attributes, "workspaceSearch");
  (summary, attributes)
}
