use std::collections::HashMap;

use super::context_memory_pack::ContextPack;
use crate::text_utils::{take_characters, take_last_characters};

const CONTEXT_OBSERVATION_BUDGET_PERCENT: usize = 45;
const MIN_CONTEXT_OBSERVATION_CHAR_BUDGET: usize = 1200;
const MAX_CONTEXT_OBSERVATION_CHAR_BUDGET: usize = 3600;

#[derive(Debug, Clone)]
pub struct PromptObservation {
  pub text: String,
  pub source_char_count: usize,
  pub budget_char_count: usize,
  pub was_truncated: bool,
}

pub fn compact_prompt_observation(content: &str, context_pack: &ContextPack) -> PromptObservation {
  let budget_char_count = observation_budget_for_context(context_pack);
  let source_char_count = content.chars().count();
  if source_char_count <= budget_char_count {
    return PromptObservation {
      text: content.to_string(),
      source_char_count,
      budget_char_count,
      was_truncated: false,
    };
  }

  let marker = format!(
    "\n\n[observation compacted: {} chars omitted]\n\n",
    source_char_count.saturating_sub(budget_char_count)
  );
  let marker_char_count = marker.chars().count();
  let available_chars = budget_char_count.saturating_sub(marker_char_count);
  let head_char_count = (available_chars * 2) / 3;
  let tail_char_count = available_chars.saturating_sub(head_char_count);
  let text = format!(
    "{}{}{}",
    take_characters(content, head_char_count),
    marker,
    take_last_characters(content, tail_char_count)
  );

  PromptObservation {
    text,
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

fn observation_budget_for_context(context_pack: &ContextPack) -> usize {
  let raw_budget = context_pack
    .context_window_tokens
    .saturating_mul(CONTEXT_OBSERVATION_BUDGET_PERCENT)
    / 100;
  raw_budget.clamp(
    MIN_CONTEXT_OBSERVATION_CHAR_BUDGET,
    MAX_CONTEXT_OBSERVATION_CHAR_BUDGET,
  )
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn prompt_observation_compacts_large_tool_output_for_small_models() {
    let context_pack = ContextPack {
      notes: vec![],
      retrieval_scores: vec![],
      context_window_tokens: 4096,
      source_note_count: 0,
      candidate_note_count: 0,
      omitted_note_count: 0,
      truncated_note_count: 0,
      estimated_char_count: 0,
      budget_char_count: 1228,
    };
    let content = format!("{}TAIL", "A".repeat(3000));

    let observation = compact_prompt_observation(&content, &context_pack);

    assert!(observation.was_truncated);
    assert_eq!(observation.budget_char_count, 1843);
    assert!(observation.text.contains("[observation compacted"));
    assert!(observation.text.ends_with("TAIL"));
    assert!(observation.text.chars().count() <= observation.budget_char_count);
  }
}
