use std::collections::HashMap;

use pith_memory::MemoryNote;
use pith_model_runtime::{GenerationCancellation, LocalModelRuntime};
use pith_tools::WebSearchResult;

use super::local_response_formatting::format_web_search_result;
use super::local_response_generation::{generate_local_summary, mark_cowork_handoff};
use crate::context_compaction::compact_prompt_observation;
use crate::context_memory_pack::{format_memory_context_prompt, pack_memory_notes_for_context};
use crate::intent_inference::WebSearchIntent;

pub(crate) const WEB_SEARCH_SOURCE_MODE: &str = "searchResultAttribution";
pub(crate) const WEB_SEARCH_SOURCE_NOTE: &str =
  "Search-result attribution only; page contents were not fetched.";
pub(crate) const WEB_SEARCH_SOURCE_SNAPSHOT_KIND: &str = "searchResults";

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
    let (summary, mut attributes) = generate_local_summary(
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
    mark_cowork_handoff(&mut attributes, "webSearchNoResults");
    return (summary, attributes);
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
  mark_cowork_handoff(&mut attributes, "webSearchSources");

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
  attributes.insert(
    "webSearchSourceMode".to_string(),
    WEB_SEARCH_SOURCE_MODE.to_string(),
  );
  attributes.insert("pageFetchPerformed".to_string(), "false".to_string());
  merge_web_search_source_snapshot_attributes(attributes, &sources);
}

fn merge_web_search_source_snapshot_attributes(
  attributes: &mut HashMap<String, String>,
  sources: &[&WebSearchResult],
) {
  let snapshot = format_web_search_source_snapshot(sources);
  attributes.insert("sourceSnapshotAvailable".to_string(), "true".to_string());
  attributes.insert(
    "sourceSnapshotKind".to_string(),
    WEB_SEARCH_SOURCE_SNAPSHOT_KIND.to_string(),
  );
  attributes.insert(
    "sourceSnapshotResultCount".to_string(),
    sources.len().to_string(),
  );
  attributes.insert(
    "sourceSnapshotHash".to_string(),
    stable_snapshot_hash(&snapshot),
  );
  attributes.insert("sourceSnapshot".to_string(), snapshot);
}

fn format_web_search_source_snapshot(sources: &[&WebSearchResult]) -> String {
  sources
    .iter()
    .enumerate()
    .map(|(index, result)| {
      format!(
        "{}. {}\nURL: {}\nSnippet: {}\nProvider: {}",
        index + 1,
        result.title,
        result.url,
        result.snippet,
        result.source
      )
    })
    .collect::<Vec<_>>()
    .join("\n\n")
}

fn stable_snapshot_hash(snapshot: &str) -> String {
  let mut hash: u64 = 0xcbf29ce484222325;
  for byte in snapshot.as_bytes() {
    hash ^= u64::from(*byte);
    hash = hash.wrapping_mul(0x100000001b3);
  }
  format!("{hash:016x}")
}

fn source_grounded_summary(summary: String, results: &[WebSearchResult]) -> String {
  let sources = results.iter().take(3).collect::<Vec<_>>();
  if sources.is_empty() {
    return summary;
  }

  let mut output = summary.trim_end().to_string();
  if !sources.iter().any(|source| output.contains(&source.url)) {
    let source_line = sources
      .iter()
      .map(|source| format!("{} ({})", source.title, source.url))
      .collect::<Vec<_>>()
      .join("; ");
    output.push_str(&format!("\n\nSources: {source_line}"));
  }
  if !output.contains(WEB_SEARCH_SOURCE_NOTE) {
    output.push_str(&format!("\n\nSource note: {WEB_SEARCH_SOURCE_NOTE}"));
  }
  output
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn web_search_source_attributes_include_search_result_snapshot() {
    let mut attributes = HashMap::new();
    merge_web_search_source_attributes(&mut attributes, &[result("Pith", "https://example.com")]);

    assert_eq!(
      attributes.get("sourceAttribution").map(String::as_str),
      Some("web_search")
    );
    assert_eq!(
      attributes.get("webSearchSourceMode").map(String::as_str),
      Some("searchResultAttribution")
    );
    assert_eq!(
      attributes.get("pageFetchPerformed").map(String::as_str),
      Some("false")
    );
    assert_eq!(
      attributes
        .get("sourceSnapshotAvailable")
        .map(String::as_str),
      Some("true")
    );
    assert_eq!(
      attributes.get("sourceSnapshotKind").map(String::as_str),
      Some("searchResults")
    );
    assert_eq!(
      attributes
        .get("sourceSnapshotResultCount")
        .map(String::as_str),
      Some("1")
    );
    assert!(attributes
      .get("sourceSnapshotHash")
      .map(|value| value.len() == 16)
      .unwrap_or(false));
    assert!(attributes
      .get("sourceSnapshot")
      .map(|value| value.contains("Snippet"))
      .unwrap_or(false));
  }

  #[test]
  fn web_search_summary_adds_source_note_even_when_urls_are_present() {
    let summary = source_grounded_summary(
      "The relevant source is https://example.com.".to_string(),
      &[result("Pith", "https://example.com")],
    );

    assert!(summary.contains("https://example.com"));
    assert!(summary.contains(WEB_SEARCH_SOURCE_NOTE));
    assert!(!summary.contains("Sources: Pith"));
  }

  fn result(title: &str, url: &str) -> WebSearchResult {
    WebSearchResult {
      title: title.to_string(),
      url: url.to_string(),
      snippet: "Snippet".to_string(),
      source: "fixture".to_string(),
    }
  }
}
