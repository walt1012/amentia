use std::collections::HashMap;

use pith_memory::MemoryNote;
use pith_model_runtime::LocalModelRuntime;
use pith_tools::ShellCommandResult;

use super::local_response_formatting::format_shell_result;
use super::local_response_generation::generate_local_summary;
use crate::context_compaction::{
  compact_prompt_observation, format_context_prompt, pack_memory_context,
};

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
