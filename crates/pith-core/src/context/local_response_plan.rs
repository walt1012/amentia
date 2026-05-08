use std::collections::HashMap;

use pith_memory::MemoryNote;
use pith_model_runtime::{GenerateRequest, LocalModelRuntime, ModelRole};
use pith_protocol::{TimelineItem, WorkspaceSummary};

use crate::context_memory_pack::{
  format_memory_context_prompt, merge_memory_context_attributes, pack_memory_notes_for_context,
};

pub(crate) fn build_plan_item(
  model_runtime: &LocalModelRuntime,
  memory_notes: &[MemoryNote],
  message: &str,
  workspace: Option<&WorkspaceSummary>,
  plan_hint: String,
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
  let result = model_runtime.generate(GenerateRequest {
    role: ModelRole::Planner,
    prompt: format!(
      "You are the local planner for Pith.\n{}\n{}\nUser request: {}\nCandidate local action: {}\nWrite one concise English sentence describing the next action Pith should take.",
      workspace_context,
      format_memory_context_prompt(&memory_context),
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
  merge_memory_context_attributes(&mut attributes, &memory_context);

  TimelineItem {
    kind: "plan".to_string(),
    title: "Plan".to_string(),
    content: result.text,
    attributes: Some(attributes),
  }
}
