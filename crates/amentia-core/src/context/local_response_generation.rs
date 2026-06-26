use std::collections::HashMap;

use amentia_model_runtime::{
  GenerateRequest, GenerationCancellation, LocalModelRuntime, ModelRole,
};

use crate::context_memory_pack::{merge_memory_context_attributes, MemoryContextPack};
use crate::context_observation::{
  compact_generation_prompt, merge_generation_prompt_attributes, merge_observation_attributes,
  PromptObservation,
};

pub(super) fn generate_local_summary(
  model_runtime: &LocalModelRuntime,
  prompt: String,
  observation_summary: String,
  memory_context: &MemoryContextPack,
  observation: Option<&PromptObservation>,
  cancellation: Option<&GenerationCancellation>,
) -> (String, HashMap<String, String>) {
  let prompt = compact_generation_prompt(
    &format!("{prompt}\nDeterministic observation:\n{observation_summary}"),
    memory_context,
  );
  let result = model_runtime.generate(GenerateRequest {
    role: ModelRole::Summarizer,
    prompt: prompt.text.clone(),
    max_tokens: 160,
    timeout: None,
    cancellation: cancellation.cloned(),
  });

  let model_status = result.status.clone();
  let mut attributes = HashMap::from([
    ("modelId".to_string(), result.model_id),
    ("modelBackend".to_string(), result.backend),
    ("modelStatus".to_string(), result.status),
  ]);
  if model_status != "ready" && model_status != "cancelled" {
    if let Some(detail) = result.detail {
      attributes.insert("modelFailureDetail".to_string(), detail);
    }
  }
  merge_memory_context_attributes(&mut attributes, memory_context);
  merge_generation_prompt_attributes(&mut attributes, &prompt);
  if let Some(observation) = observation {
    merge_observation_attributes(&mut attributes, observation);
  }

  (result.text, attributes)
}

pub(super) fn mark_cowork_handoff(attributes: &mut HashMap<String, String>, handoff_kind: &str) {
  attributes.insert("responseRole".to_string(), "coworkHandoff".to_string());
  attributes.insert("handoffKind".to_string(), handoff_kind.to_string());
}
