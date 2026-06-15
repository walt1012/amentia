use std::collections::HashMap;

use pith_memory::MemoryNote;
use pith_model_runtime::LocalModelRuntime;

use super::local_response_generation::generate_local_summary;
use crate::context_memory_pack::{format_memory_context_prompt, pack_memory_notes_for_context};

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
  let memory_context =
    pack_memory_notes_for_context(model_runtime, memory_notes, Some(workspace_name), &query);
  let observation_summary = if action == "run_shell" {
    let command = command.unwrap_or_default();
    format!(
      "Pith skipped the shell command `{}` because the approval was denied.",
      command
    )
  } else if action == "run_plugin_command" {
    let command = command.unwrap_or_default();
    format!(
      "Pith skipped the plugin action `{}` because the approval was denied.",
      command
    )
  } else {
    format!(
      "Pith skipped writing {} because the approval was denied.",
      relative_path
    )
  };
  let prompt = format!(
    "You are Pith, a concise local cowork agent. Summarize a denied approval in one sentence.\nWorkspace: {workspace_name}\n{}\nAction: {}\nTarget: {}\nCommand: {}",
    format_memory_context_prompt(&memory_context),
    action,
    relative_path,
    command.unwrap_or_default()
  );

  generate_local_summary(
    model_runtime,
    prompt,
    observation_summary,
    &memory_context,
    None,
    None,
  )
}
