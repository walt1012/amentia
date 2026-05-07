use std::collections::HashMap;

use pith_memory::MemoryNote;
use pith_model_runtime::LocalModelRuntime;
use pith_tools::{DirectoryEntry, ReadFileResult, SearchMatch, ShellCommandResult};

use super::local_response_formatting::{
  format_directory_result, format_search_result, format_shell_result,
};
use super::local_response_generation::generate_local_summary;
use crate::context_compaction::{
  compact_prompt_observation, format_context_prompt, pack_memory_context,
};

pub(crate) fn summarize_file_result(
  model_runtime: &LocalModelRuntime,
  memory_notes: &[MemoryNote],
  thread_title: &str,
  workspace_name: &str,
  result: &ReadFileResult,
) -> (String, HashMap<String, String>) {
  let context_pack = pack_memory_context(
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
  let observation = compact_prompt_observation(&result.content, &context_pack);
  let prompt = format!(
    "You are Pith, a concise local coding agent. Summarize a file inspection in one or two sentences.\nThread: {thread_title}\nWorkspace: {workspace_name}\n{}\nFile: {}\nPreview:\n{}",
    format_context_prompt(&context_pack),
    result.relative_path,
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

pub(crate) fn summarize_directory_result(
  model_runtime: &LocalModelRuntime,
  memory_notes: &[MemoryNote],
  thread_title: &str,
  workspace_name: &str,
  entries: &[DirectoryEntry],
) -> (String, HashMap<String, String>) {
  let context_pack = pack_memory_context(
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
        format_context_prompt(&context_pack)
      ),
      format!(
        "Pith inspected {} for {} and found an empty root directory.",
        workspace_name, thread_title
      ),
      &context_pack,
      None,
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
  let observation = compact_prompt_observation(&format_directory_result(entries), &context_pack);
  let prompt = format!(
    "You are Pith, a concise local coding agent. Summarize a root directory inspection in one or two sentences.\nThread: {thread_title}\nWorkspace: {workspace_name}\n{}\nEntries:\n{}",
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

pub(crate) fn summarize_shell_result(
  model_runtime: &LocalModelRuntime,
  memory_notes: &[MemoryNote],
  workspace_name: &str,
  result: &ShellCommandResult,
) -> (String, HashMap<String, String>) {
  let context_pack = pack_memory_context(
    model_runtime,
    memory_notes,
    Some(workspace_name),
    &result.command,
  );
  let sandbox_summary = if result.sandbox.active {
    "native sandbox active"
  } else {
    "native sandbox limited"
  };
  let observation_summary = if result.exit_code == 0 {
    format!(
      "Pith ran `{}` in {} with {} and it finished successfully.",
      result.command, workspace_name, sandbox_summary
    )
  } else {
    format!(
      "Pith ran `{}` in {} with {} and it exited with code {}.",
      result.command, workspace_name, sandbox_summary, result.exit_code
    )
  };
  let observation = compact_prompt_observation(&format_shell_result(result), &context_pack);
  let prompt = format!(
    "You are Pith, a concise local coding agent. Summarize a shell command result in one or two sentences.\nWorkspace: {workspace_name}\n{}\nResult Preview:\n{}",
    format_context_prompt(&context_pack),
    observation.text
  );

  let (summary, mut attributes) = generate_local_summary(
    model_runtime,
    prompt,
    observation_summary,
    &context_pack,
    Some(&observation),
  );
  attributes.extend(result.sandbox.attributes());

  (summary, attributes)
}
