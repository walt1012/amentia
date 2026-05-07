use std::collections::HashMap;

use pith_model_runtime::{GenerateRequest, LocalModelRuntime, ModelRole};

use crate::context_compaction::{
  merge_context_pack_attributes, merge_observation_attributes, ContextPack, PromptObservation,
};

pub(super) fn generate_local_summary(
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
