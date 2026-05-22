use std::collections::HashMap;

use pith_protocol::TimelineItem;

use super::context_memory_pack::MemoryContextPack;
use crate::text_utils::{take_characters, take_last_characters};

const CONTEXT_OBSERVATION_BUDGET_PERCENT: usize = 45;
const MIN_CONTEXT_OBSERVATION_CHAR_BUDGET: usize = 1200;
const MAX_CONTEXT_OBSERVATION_CHAR_BUDGET: usize = 3600;
const GENERATION_PROMPT_BUDGET_PERCENT: usize = 85;
const MIN_GENERATION_PROMPT_CHAR_BUDGET: usize = 1800;
const MAX_GENERATION_PROMPT_CHAR_BUDGET: usize = 7200;
const PRIOR_OBSERVATION_CHAR_BUDGET: usize = 1800;

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

#[derive(Debug, Clone)]
pub struct PriorObservationContext {
  pub text: String,
  pub observation_count: usize,
  pub source_char_count: usize,
  pub budget_char_count: usize,
  pub was_truncated: bool,
  pub relative_paths: Vec<String>,
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

pub fn collect_prior_observation_context(items: &[TimelineItem]) -> PriorObservationContext {
  let mut blocks = Vec::new();
  let mut relative_paths = Vec::new();

  for item in items.iter().filter(|item| is_prior_observation_item(item)) {
    let attributes = item.attributes.as_ref();
    if let Some(path) = attributes
      .and_then(|attributes| attributes.get("relativePath"))
      .filter(|path| !path.is_empty())
    {
      if !relative_paths.iter().any(|known_path| known_path == path) {
        relative_paths.push(path.clone());
      }
    }

    let tool = attributes
      .and_then(|attributes| attributes.get("tool"))
      .cloned()
      .unwrap_or_else(|| item.title.clone());
    let content = item.content.trim();
    if content.is_empty() {
      blocks.push(format!("- {tool}: [empty observation]"));
    } else {
      blocks.push(format!("- {tool}:\n{content}"));
    }
  }

  let source_text = blocks.join("\n\n");
  let source_char_count = source_text.chars().count();
  let (text, was_truncated) = if source_char_count > PRIOR_OBSERVATION_CHAR_BUDGET {
    (
      compact_text_with_marker(
        &source_text,
        PRIOR_OBSERVATION_CHAR_BUDGET,
        "prior observations compacted",
        source_char_count,
      ),
      true,
    )
  } else {
    (source_text, false)
  };

  PriorObservationContext {
    text,
    observation_count: blocks.len(),
    source_char_count,
    budget_char_count: PRIOR_OBSERVATION_CHAR_BUDGET,
    was_truncated,
    relative_paths,
  }
}

pub fn merge_prior_observation_attributes(
  attributes: &mut HashMap<String, String>,
  prior_observations: &PriorObservationContext,
) {
  if prior_observations.observation_count == 0 {
    return;
  }

  attributes.insert(
    "priorObservationCount".to_string(),
    prior_observations.observation_count.to_string(),
  );
  attributes.insert(
    "priorObservationSourceChars".to_string(),
    prior_observations.source_char_count.to_string(),
  );
  attributes.insert(
    "priorObservationBudgetChars".to_string(),
    prior_observations.budget_char_count.to_string(),
  );
  attributes.insert(
    "priorObservationTruncated".to_string(),
    prior_observations.was_truncated.to_string(),
  );
  if !prior_observations.relative_paths.is_empty() {
    attributes.insert(
      "priorObservationPaths".to_string(),
      prior_observations.relative_paths.join("\n"),
    );
  }
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

fn is_prior_observation_item(item: &TimelineItem) -> bool {
  matches!(
    item.kind.as_str(),
    "toolResult" | "pluginResult" | "diffArtifact"
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

  #[test]
  fn prior_observation_context_collects_completed_tool_results() {
    let items = vec![
      TimelineItem {
        kind: "toolResult".to_string(),
        title: "read_file result".to_string(),
        content: "File: README.md\n\nOverview".to_string(),
        attributes: Some(HashMap::from([
          ("tool".to_string(), "read_file".to_string()),
          ("relativePath".to_string(), "README.md".to_string()),
        ])),
      },
      TimelineItem {
        kind: "assistantMessage".to_string(),
        title: "Assistant".to_string(),
        content: "Ignore this.".to_string(),
        attributes: None,
      },
    ];

    let context = collect_prior_observation_context(&items);

    assert_eq!(context.observation_count, 1);
    assert_eq!(context.relative_paths, vec!["README.md".to_string()]);
    assert!(context.text.contains("Overview"));
  }
}
