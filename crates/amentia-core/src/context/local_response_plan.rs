use std::collections::HashMap;

use amentia_memory::MemoryNote;
use amentia_model_runtime::{
  GenerateRequest, GenerationCancellation, LocalModelRuntime, ModelRole,
};
use amentia_protocol::{TimelineItem, WorkspaceSummary};

use crate::context_memory_pack::{
  format_memory_context_prompt, merge_memory_context_attributes, pack_memory_notes_for_context,
};
use crate::context_observation::{compact_generation_prompt, merge_generation_prompt_attributes};
use crate::context_plugin_skill_pack::{
  format_plugin_skill_context_prompt, merge_plugin_skill_context_attributes, PluginSkillContextPack,
};

pub(crate) fn build_plan_item(
  model_runtime: &LocalModelRuntime,
  memory_notes: &[MemoryNote],
  plugin_skill_context: &PluginSkillContextPack,
  message: &str,
  workspace: Option<&WorkspaceSummary>,
  plan_hint: String,
  cancellation: Option<&GenerationCancellation>,
) -> TimelineItem {
  let memory_context = pack_memory_notes_for_context(
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
  let planner_context = format!(
    "{}\n{}",
    format_memory_context_prompt(&memory_context),
    format_plugin_skill_context_prompt(plugin_skill_context)
  );
  let prompt = compact_generation_prompt(
    &format!(
      "You are the local planner for Amentia.\n{workspace_context}\n{}\nUser request: {message}\nCandidate local action: {plan_hint}\nWrite one concise English sentence describing the next action Amentia should take.",
      planner_context
    ),
    &memory_context,
  );
  let result = model_runtime.generate(GenerateRequest {
    role: ModelRole::Planner,
    prompt: prompt.text.clone(),
    max_tokens: 80,
    timeout: None,
    cancellation: cancellation.cloned(),
  });
  let model_status = result.status.clone();
  let mut attributes = HashMap::from([
    ("responseRole".to_string(), "planner".to_string()),
    ("modelId".to_string(), result.model_id),
    ("modelBackend".to_string(), result.backend),
    ("modelStatus".to_string(), result.status),
  ]);
  if model_status != "ready" && model_status != "cancelled" {
    if let Some(detail) = result.detail {
      attributes.insert("modelFailureDetail".to_string(), detail);
    }
  }
  if let Some(workspace) = workspace {
    attributes.insert(
      "workspaceDisplayName".to_string(),
      workspace.display_name.clone(),
    );
  }
  merge_memory_context_attributes(&mut attributes, &memory_context);
  merge_plugin_skill_context_attributes(&mut attributes, plugin_skill_context);
  merge_generation_prompt_attributes(&mut attributes, &prompt);

  TimelineItem {
    kind: "plan".to_string(),
    title: "Plan".to_string(),
    content: result.text,
    attributes: Some(attributes),
  }
}
