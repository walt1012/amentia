use std::collections::HashMap;

use pith_memory::MemoryNote;
use pith_model_runtime::{GenerationCancellation, LocalModelRuntime};
use pith_tools::WebSearchResult;

use super::local_response_formatting::format_web_search_result;
use super::local_response_generation::generate_local_summary;
use crate::context_compaction::compact_prompt_observation;
use crate::context_memory_pack::{format_memory_context_prompt, pack_memory_notes_for_context};
use crate::intent_inference::WebSearchIntent;

pub(crate) fn summarize_declined_web_search_candidate(
  model_runtime: &LocalModelRuntime,
  memory_notes: &[MemoryNote],
  thread_title: &str,
  user_message: &str,
  intent: &WebSearchIntent,
  cancellation: Option<&GenerationCancellation>,
) -> (String, HashMap<String, String>) {
  let memory_context =
    pack_memory_notes_for_context(model_runtime, memory_notes, None, user_message);
  let observation_summary = format!(
    "The local planner declined web_search for candidate query \"{}\" because {}.",
    intent.query, intent.routing_reason
  );
  let prompt = format!(
    "You are Pith, a concise local agent. Answer the user directly without using web_search.\n\
     If the request depends on current public facts, be transparent that no web search was used.\n\
     Thread: {thread_title}\n{}\nUser request: {user_message}",
    format_memory_context_prompt(&memory_context)
  );

  generate_local_summary(
    model_runtime,
    prompt,
    observation_summary,
    &memory_context,
    None,
    cancellation,
  )
}

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

  let (summary, mut attributes) = generate_local_summary(
    model_runtime,
    prompt,
    observation_summary,
    &memory_context,
    Some(&observation),
    cancellation,
  );
  merge_web_search_source_attributes(&mut attributes, results);

  (source_grounded_summary(summary, results), attributes)
}

fn merge_web_search_source_attributes(
  attributes: &mut HashMap<String, String>,
  results: &[WebSearchResult],
) {
  let sources = results.iter().take(3).collect::<Vec<_>>();
  attributes.insert("sourceCount".to_string(), sources.len().to_string());
  attributes.insert(
    "sourceTitles".to_string(),
    sources
      .iter()
      .map(|result| result.title.as_str())
      .collect::<Vec<_>>()
      .join("\n"),
  );
  attributes.insert(
    "sourceUrls".to_string(),
    sources
      .iter()
      .map(|result| result.url.as_str())
      .collect::<Vec<_>>()
      .join("\n"),
  );
  attributes.insert("sourceAttribution".to_string(), "web_search".to_string());
}

fn source_grounded_summary(summary: String, results: &[WebSearchResult]) -> String {
  let sources = results.iter().take(3).collect::<Vec<_>>();
  if sources.is_empty() || sources.iter().any(|source| summary.contains(&source.url)) {
    return summary;
  }

  let source_line = sources
    .iter()
    .map(|source| format!("{} ({})", source.title, source.url))
    .collect::<Vec<_>>()
    .join("; ");
  format!("{}\n\nSources: {}", summary.trim_end(), source_line)
}
