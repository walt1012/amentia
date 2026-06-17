use std::collections::HashMap;

use amentia_memory::MemoryNote;
use amentia_model_runtime::{GenerationCancellation, LocalModelRuntime};
use amentia_tools::ShellCommandResult;

use super::local_response_formatting::format_shell_result;
use super::local_response_generation::generate_local_summary;
use crate::context_compaction::compact_prompt_observation;
use crate::context_memory_pack::{format_memory_context_prompt, pack_memory_notes_for_context};

pub(crate) fn summarize_shell_result(
  model_runtime: &LocalModelRuntime,
  memory_notes: &[MemoryNote],
  workspace_name: &str,
  result: &ShellCommandResult,
  cancellation: Option<&GenerationCancellation>,
) -> (String, HashMap<String, String>) {
  let memory_context = pack_memory_notes_for_context(
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
  let observation_summary = if result.cancelled {
    format!(
      "Amentia ran `{}` in {} with {} and it was cancelled.",
      result.command, workspace_name, sandbox_summary
    )
  } else if result.exit_code == 0 {
    format!(
      "Amentia ran `{}` in {} with {} and it finished successfully.",
      result.command, workspace_name, sandbox_summary
    )
  } else {
    format!(
      "Amentia ran `{}` in {} with {} and it exited with code {}.",
      result.command, workspace_name, sandbox_summary, result.exit_code
    )
  };
  let observation = compact_prompt_observation(&format_shell_result(result), &memory_context);
  let prompt = format!(
    "You are Amentia, a concise local cowork agent. Summarize a shell command result in one or two sentences.\nWorkspace: {workspace_name}\n{}\nResult Preview:\n{}",
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
  attributes.extend(result.sandbox.attributes());
  attributes.extend(result.output_context.attributes());

  (summary, attributes)
}
