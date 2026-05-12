use std::collections::HashMap;

use super::context_memory_pack::MemoryContextPack;
use crate::text_utils::{take_characters, take_last_characters};

const CONTEXT_OBSERVATION_BUDGET_PERCENT: usize = 45;
const MIN_CONTEXT_OBSERVATION_CHAR_BUDGET: usize = 1200;
const MAX_CONTEXT_OBSERVATION_CHAR_BUDGET: usize = 3600;
const GENERATION_PROMPT_BUDGET_PERCENT: usize = 85;
const MIN_GENERATION_PROMPT_CHAR_BUDGET: usize = 1800;
const MAX_GENERATION_PROMPT_CHAR_BUDGET: usize = 7200;

#[derive(Debug, Clone)]
pub struct PromptObservation {
  pub text: String,
  pub source_char_count: usize,
  pub budget_char_count: usize,
  pub was_truncated: bool,
}

#[derive(Debug, Clone)]
pub struct GenerationPromptEnvelope {
  pub text: String,
  pub source_char_count: usize,
  pub budget_char_count: usize,
  pub was_truncated: bool,
}

pub fn compact_prompt_observation(
  content: &str,
  memory_context: &MemoryContextPack,
) -> PromptObservation {
  let budget_char_count = observation_budget_for_context(memory_context);
  let source_char_count = content.chars().count();
  if source_char_count <= budget_char_count {
    return PromptObservation {
      text: content.to_string(),
      source_char_count,
      budget_char_count,
      was_truncated: false,
    };
  }

  let text = compact_text_with_marker(
    content,
    budget_char_count,
    "observation compacted",
    source_char_count,
  );

  PromptObservation {
    text,
    source_char_count,
    budget_char_count,
    was_truncated: true,
  }
}

pub fn compact_generation_prompt(
  prompt: &str,
  memory_context: &MemoryContextPack,
) -> GenerationPromptEnvelope {
  let budget_char_count = generation_prompt_budget_for_context(memory_context);
  let source_char_count = prompt.chars().count();
  if source_char_count <= budget_char_count {
    return GenerationPromptEnvelope {
      text: prompt.to_string(),
      source_char_count,
      budget_char_count,
      was_truncated: false,
    };
  }

  GenerationPromptEnvelope {
    text: compact_text_with_marker(
      prompt,
      budget_char_count,
      "prompt compacted",
      source_char_count,
    ),
    source_char_count,
    budget_char_count,
    was_truncated: true,
  }
}

pub fn merge_observation_attributes(
  attributes: &mut HashMap<String, String>,
  observation: &PromptObservation,
) {
  attributes.insert(
    "observationSourceChars".to_string(),
    observation.source_char_count.to_string(),
  );
  attributes.insert(
    "observationBudgetChars".to_string(),
    observation.budget_char_count.to_string(),
  );
  attributes.insert(
    "observationTruncated".to_string(),
    observation.was_truncated.to_string(),
  );
}

pub fn merge_generation_prompt_attributes(
  attributes: &mut HashMap<String, String>,
  prompt: &GenerationPromptEnvelope,
) {
  attributes.insert(
    "promptSourceChars".to_string(),
    prompt.source_char_count.to_string(),
  );
  attributes.insert(
    "promptBudgetChars".to_string(),
    prompt.budget_char_count.to_string(),
  );
  attributes.insert(
    "promptTruncated".to_string(),
    prompt.was_truncated.to_string(),
  );
}

fn observation_budget_for_context(memory_context: &MemoryContextPack) -> usize {
  let raw_budget = memory_context
    .context_window_tokens
    .saturating_mul(CONTEXT_OBSERVATION_BUDGET_PERCENT)
    / 100;
  raw_budget.clamp(
    MIN_CONTEXT_OBSERVATION_CHAR_BUDGET,
    MAX_CONTEXT_OBSERVATION_CHAR_BUDGET,
  )
}

fn generation_prompt_budget_for_context(memory_context: &MemoryContextPack) -> usize {
  let raw_budget = memory_context
    .context_window_tokens
    .saturating_mul(GENERATION_PROMPT_BUDGET_PERCENT)
    / 100;
  raw_budget.clamp(
    MIN_GENERATION_PROMPT_CHAR_BUDGET,
    MAX_GENERATION_PROMPT_CHAR_BUDGET,
  )
}

fn compact_text_with_marker(
  content: &str,
  budget_char_count: usize,
  label: &str,
  source_char_count: usize,
) -> String {
  let marker = format!(
    "\n\n[{label}: {} chars omitted]\n\n",
    source_char_count.saturating_sub(budget_char_count)
  );
  let marker_char_count = marker.chars().count();
  let available_chars = budget_char_count.saturating_sub(marker_char_count);
  let head_char_count = (available_chars * 2) / 3;
  let tail_char_count = available_chars.saturating_sub(head_char_count);

  format!(
    "{}{}{}",
    take_characters(content, head_char_count),
    marker,
    take_last_characters(content, tail_char_count)
  )
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn prompt_observation_compacts_large_tool_output_for_small_models() {
    let memory_context = MemoryContextPack {
      notes: vec![],
      memory_ranking_scores: vec![],
      context_window_tokens: 4096,
      source_note_count: 0,
      candidate_note_count: 0,
      omitted_note_count: 0,
      truncated_note_count: 0,
      estimated_char_count: 0,
      budget_char_count: 1228,
    };
    let content = format!("{}TAIL", "A".repeat(3000));

    let observation = compact_prompt_observation(&content, &memory_context);

    assert!(observation.was_truncated);
    assert_eq!(observation.budget_char_count, 1843);
    assert!(observation.text.contains("[observation compacted"));
    assert!(observation.text.ends_with("TAIL"));
    assert!(observation.text.chars().count() <= observation.budget_char_count);
  }

  #[test]
  fn generation_prompt_envelope_caps_combined_context_for_small_models() {
    let memory_context = MemoryContextPack {
      notes: vec![],
      memory_ranking_scores: vec![],
      context_window_tokens: 4096,
      source_note_count: 0,
      candidate_note_count: 0,
      omitted_note_count: 0,
      truncated_note_count: 0,
      estimated_char_count: 0,
      budget_char_count: 1228,
    };
    let prompt = format!("Instruction\n{}TAIL", "x".repeat(5000));

    let envelope = compact_generation_prompt(&prompt, &memory_context);

    assert!(envelope.was_truncated);
    assert_eq!(envelope.budget_char_count, 3481);
    assert!(envelope.text.contains("[prompt compacted"));
    assert!(envelope.text.ends_with("TAIL"));
    assert!(envelope.text.chars().count() <= envelope.budget_char_count);
  }
}
