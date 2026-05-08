use std::collections::HashMap;

use pith_memory::MemoryNote;
use pith_model_runtime::{GenerationCancellation, LocalModelRuntime};
use pith_tools::WebSearchResult;

use super::local_response_formatting::format_web_search_result;
use super::local_response_generation::generate_local_summary;
use crate::context_compaction::compact_prompt_observation;
use crate::context_memory_pack::{format_memory_context_prompt, pack_memory_notes_for_context};

pub(crate) fn summarize_web_search_result(
  model_runtime: &LocalModelRuntime,
  memory_notes: &[MemoryNote],
  thread_title: &str,
  query: &str,
  results: &[WebSearchResult],
  cancellation: Option<&GenerationCancellation>,
) -> (String, HashMap<String, String>) {
  let memory_context = pack_memory_notes_for_context(model_runtime, memory_notes, None, query);
  if results.is_empty() {
    return generate_local_summary(
      model_runtime,
      format!(
        "You are Pith, a concise local agent. Summarize a web search with no results.\nThread: {thread_title}\n{}\nQuery: {query}",
        format_memory_context_prompt(&memory_context)
      ),
      format!("Pith searched the web and found no results for \"{}\".", query),
      &memory_context,
      None,
      cancellation,
    );
  }

  let preview = results
    .iter()
    .take(3)
    .map(|entry| format!("{} ({})", entry.title, entry.url))
    .collect::<Vec<_>>()
    .join(", ");
  let observation_summary = format!(
    "Pith searched the web and found {} results for \"{}\", including {}.",
    results.len(),
    query,
    preview
  );
  let observation =
    compact_prompt_observation(&format_web_search_result(query, results), &memory_context);
  let prompt = format!(
    "You are Pith, a concise local agent. Summarize these web search results in one or two sentences.\nThread: {thread_title}\n{}\nQuery: {query}\nResults:\n{}",
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
