use std::collections::HashMap;

use pith_memory::MemoryNote;
use pith_model_runtime::{GenerateRequest, LocalModelRuntime, ModelRole};
use pith_protocol::{TimelineItem, WorkspaceSummary};
use pith_tools::{DirectoryEntry, ReadFileResult, SearchMatch, ShellCommandResult};

use crate::context_compaction::{
  compact_prompt_observation, format_context_prompt, merge_context_pack_attributes,
  merge_observation_attributes, pack_memory_context, ContextPack, PromptObservation,
};

pub(crate) fn build_plan_item(
  model_runtime: &LocalModelRuntime,
  memory_notes: &[MemoryNote],
  message: &str,
  workspace: Option<&WorkspaceSummary>,
  plan_hint: String,
) -> TimelineItem {
  let context_pack = pack_memory_context(
    model_runtime,
    memory_notes,
    workspace.map(|entry| entry.display_name.as_str()),
    message,
  );
  let workspace_context = workspace
    .map(|workspace| {
      format!(
        "Workspace: {} at {}.",
        workspace.display_name, workspace.root_path
      )
    })
    .unwrap_or_else(|| "Workspace: unavailable.".to_string());
  let result = model_runtime.generate(GenerateRequest {
    role: ModelRole::Planner,
    prompt: format!(
      "You are the local planner for Pith.\n{}\n{}\nUser request: {}\nCandidate local action: {}\nWrite one concise English sentence describing the next action Pith should take.",
      workspace_context,
      format_context_prompt(&context_pack),
      message,
      plan_hint
    ),
    max_tokens: 80,
  });
  let mut attributes = HashMap::from([
    ("responseRole".to_string(), "planner".to_string()),
    ("modelId".to_string(), result.model_id),
    ("modelBackend".to_string(), result.backend),
    ("modelStatus".to_string(), result.status),
  ]);
  if let Some(workspace) = workspace {
    attributes.insert(
      "workspaceDisplayName".to_string(),
      workspace.display_name.clone(),
    );
  }
  merge_context_pack_attributes(&mut attributes, &context_pack);

  TimelineItem {
    kind: "plan".to_string(),
    title: "Plan".to_string(),
    content: result.text,
    attributes: Some(attributes),
  }
}

pub(crate) fn format_file_result(result: &ReadFileResult) -> String {
  if result.is_truncated {
    format!(
      "File: {}\n\n{}\n\n[output truncated at 4096 bytes]",
      result.relative_path, result.content
    )
  } else {
    format!("File: {}\n\n{}", result.relative_path, result.content)
  }
}

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

pub(crate) fn format_directory_result(entries: &[DirectoryEntry]) -> String {
  if entries.is_empty() {
    return "The directory is empty.".to_string();
  }

  entries
    .iter()
    .map(|entry| format!("[{}] {}", entry.entry_type, entry.relative_path))
    .collect::<Vec<_>>()
    .join("\n")
}

pub(crate) fn format_search_result(query: &str, matches: &[SearchMatch]) -> String {
  if matches.is_empty() {
    return format!("No matches found for \"{}\".", query);
  }

  matches
    .iter()
    .map(|entry| {
      format!(
        "{}:{}: {}",
        entry.relative_path, entry.line_number, entry.line
      )
    })
    .collect::<Vec<_>>()
    .join("\n")
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

pub(crate) fn format_shell_result(result: &ShellCommandResult) -> String {
  let stdout = if result.stdout.trim().is_empty() {
    "[no stdout]".to_string()
  } else {
    result.stdout.clone()
  };
  let stderr = if result.stderr.trim().is_empty() {
    "[no stderr]".to_string()
  } else {
    result.stderr.clone()
  };
  let truncation_note = if result.was_truncated {
    "\n\n[output truncated]"
  } else {
    ""
  };
  let timeout_note = if result.timed_out {
    "\n\n[command timed out]"
  } else {
    ""
  };

  format!(
    "Command: {}\nExit Code: {}\n\nstdout:\n{}\n\nstderr:\n{}{}{}",
    result.command, result.exit_code, stdout, stderr, truncation_note, timeout_note
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
  let observation_summary = if result.exit_code == 0 {
    format!(
      "Pith ran `{}` in {} and it finished successfully.",
      result.command, workspace_name
    )
  } else {
    format!(
      "Pith ran `{}` in {} and it exited with code {}.",
      result.command, workspace_name, result.exit_code
    )
  };
  let observation = compact_prompt_observation(&format_shell_result(result), &context_pack);
  let prompt = format!(
    "You are Pith, a concise local coding agent. Summarize a shell command result in one or two sentences.\nWorkspace: {workspace_name}\n{}\nResult Preview:\n{}",
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

pub(crate) fn summarize_denied_approval(
  model_runtime: &LocalModelRuntime,
  memory_notes: &[MemoryNote],
  workspace_name: &str,
  action: &str,
  relative_path: &str,
  command: Option<&str>,
) -> (String, HashMap<String, String>) {
  let query = command
    .map(str::to_string)
    .unwrap_or_else(|| format!("{action} {relative_path}"));
  let context_pack = pack_memory_context(model_runtime, memory_notes, Some(workspace_name), &query);
  let observation_summary = if action == "run_shell" {
    let command = command.unwrap_or_default();
    format!(
      "Pith skipped the shell command `{}` because the approval was denied.",
      command
    )
  } else {
    format!(
      "Pith skipped writing {} because the approval was denied.",
      relative_path
    )
  };
  let prompt = format!(
    "You are Pith, a concise local coding agent. Summarize a denied approval in one sentence.\nWorkspace: {workspace_name}\n{}\nAction: {}\nTarget: {}\nCommand: {}",
    format_context_prompt(&context_pack),
    action,
    relative_path,
    command.unwrap_or_default()
  );

  generate_local_summary(
    model_runtime,
    prompt,
    observation_summary,
    &context_pack,
    None,
  )
}

fn generate_local_summary(
  model_runtime: &LocalModelRuntime,
  prompt: String,
  observation_summary: String,
  context_pack: &ContextPack,
  observation: Option<&PromptObservation>,
) -> (String, HashMap<String, String>) {
  let result = model_runtime.generate(GenerateRequest {
    role: ModelRole::Summarizer,
    prompt: format!("{prompt}\nDeterministic observation:\n{observation_summary}"),
    max_tokens: 160,
  });

  let mut attributes = HashMap::from([
    ("modelId".to_string(), result.model_id),
    ("modelBackend".to_string(), result.backend),
    ("modelStatus".to_string(), result.status),
  ]);
  merge_context_pack_attributes(&mut attributes, context_pack);
  if let Some(observation) = observation {
    merge_observation_attributes(&mut attributes, observation);
  }

  (result.text, attributes)
}
