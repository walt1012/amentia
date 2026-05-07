use std::collections::HashMap;

use pith_memory::MemoryNote;
use pith_model_runtime::LocalModelRuntime;
use pith_tools::SearchMatch;

use super::local_response_formatting::format_search_result;
use super::local_response_generation::generate_local_summary;
use crate::context_compaction::{
  compact_prompt_observation, format_context_prompt, pack_memory_context,
};

pub(crate) fn summarize_search_result(
  model_runtime: &LocalModelRuntime,
  memory_notes: &[MemoryNote],
  thread_title: &str,
  workspace_name: &str,
  query: &str,
  matches: &[SearchMatch],
) -> (String, HashMap<String, String>) {
  let context_pack = pack_memory_context(model_runtime, memory_notes, Some(workspace_name), query);
  if matches.is_empty() {
    return generate_local_summary(
      model_runtime,
      format!(
        "You are Pith, a concise local coding agent. Summarize a search with no matches.\nThread: {thread_title}\nWorkspace: {workspace_name}\n{}\nQuery: {query}",
        format_context_prompt(&context_pack)
      ),
      format!(
        "Pith searched {} for {} and found no matches for \"{}\".",
        workspace_name, thread_title, query
      ),
      &context_pack,
      None,
    );
  }

  let preview = matches
    .iter()
    .take(3)
    .map(|entry| format!("{}:{}", entry.relative_path, entry.line_number))
    .collect::<Vec<_>>()
    .join(", ");

  let observation_summary = format!(
    "Pith searched {} for {} and found {} matches for \"{}\", including {}.",
    workspace_name,
    thread_title,
    matches.len(),
    query,
    preview
  );
  let observation =
    compact_prompt_observation(&format_search_result(query, matches), &context_pack);
  let prompt = format!(
    "You are Pith, a concise local coding agent. Summarize a workspace search in one or two sentences.\nThread: {thread_title}\nWorkspace: {workspace_name}\n{}\nQuery: {query}\nMatches:\n{}",
    format_context_prompt(&context_pack),
    observation.text
  );

  generate_local_summary(
    model_runtime,
    prompt,
    observation_summary,
    &context_pack,
    Some(&observation),
  )
}
