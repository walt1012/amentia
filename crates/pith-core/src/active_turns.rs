use std::collections::HashMap;
use std::time::Instant;

use pith_protocol::TimelineItem;

use crate::text_utils::take_characters;

#[derive(Debug, Clone)]
pub(crate) struct ActiveTurn {
  pub(crate) id: String,
  pub(crate) thread_id: String,
  pub(crate) full_content: String,
  pub(crate) emitted_chars: usize,
  pub(crate) total_chars: usize,
  started_at: Instant,
}

pub(crate) fn start_streaming_assistant_turn(
  thread_id: &str,
  turn_id: &str,
  items: &mut Vec<TimelineItem>,
  full_content: String,
  mut attributes: HashMap<String, String>,
) -> Option<ActiveTurn> {
  let initial_chars = 48.min(full_content.chars().count());
  let total_chars = full_content.chars().count();
  let initial_content = take_characters(&full_content, initial_chars);
  let is_complete = initial_chars >= total_chars;
  let streaming_status = if is_complete {
    "completed"
  } else {
    "in_progress"
  };
  attributes.insert("turnId".to_string(), turn_id.to_string());
  attributes.insert("streamingStatus".to_string(), streaming_status.to_string());
  attributes.insert("streamedCharacters".to_string(), initial_chars.to_string());
  attributes.insert("totalCharacters".to_string(), total_chars.to_string());
  attributes.insert("responseRole".to_string(), "summarizer".to_string());

  items.push(TimelineItem {
    kind: "assistantMessage".to_string(),
    title: "Assistant".to_string(),
    content: initial_content,
    attributes: Some(attributes),
  });

  if is_complete {
    return None;
  }

  Some(ActiveTurn {
    id: turn_id.to_string(),
    thread_id: thread_id.to_string(),
    full_content,
    emitted_chars: initial_chars,
    total_chars,
    started_at: Instant::now(),
  })
}

pub(crate) fn compute_streamed_char_count(turn: &ActiveTurn) -> usize {
  let elapsed_steps = (turn.started_at.elapsed().as_millis() / 180) as usize;
  let base_chars = 48;
  let step_chars = 72;

  base_chars + elapsed_steps * step_chars
}

pub(crate) fn update_streaming_item(
  items: &mut [TimelineItem],
  turn_id: &str,
  content: &str,
  streaming_status: &str,
  streamed_chars: usize,
  total_chars: usize,
) {
  let Some(item) = items.iter_mut().rev().find(|item| {
    item.kind == "assistantMessage"
      && item
        .attributes
        .as_ref()
        .and_then(|attributes| attributes.get("turnId"))
        .map(|value| value == turn_id)
        .unwrap_or(false)
  }) else {
    return;
  };

  item.content = content.to_string();
  let mut attributes = item.attributes.clone().unwrap_or_default();
  attributes.insert("turnId".to_string(), turn_id.to_string());
  attributes.insert("streamingStatus".to_string(), streaming_status.to_string());
  attributes.insert("streamedCharacters".to_string(), streamed_chars.to_string());
  attributes.insert("totalCharacters".to_string(), total_chars.to_string());
  item.attributes = Some(attributes);
}

pub(crate) fn active_turn_id_for_thread(
  active_turns: &HashMap<String, ActiveTurn>,
  thread_id: &str,
) -> Option<String> {
  active_turns
    .values()
    .find(|turn| turn.thread_id == thread_id)
    .map(|turn| turn.id.clone())
}

pub(crate) fn streaming_progress_label(streamed_chars: usize, total_chars: usize) -> String {
  if total_chars == 0 {
    return "0%".to_string();
  }

  let percentage = ((streamed_chars as f64 / total_chars as f64) * 100.0).round() as usize;
  format!("{}%", percentage.min(100))
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn update_streaming_item_preserves_existing_attributes() {
    let mut items = vec![TimelineItem {
      kind: "assistantMessage".to_string(),
      title: "Assistant".to_string(),
      content: "old".to_string(),
      attributes: Some(HashMap::from([(
        "modelStatus".to_string(),
        "ready".to_string(),
      )])),
    }];

    update_streaming_item(&mut items, "turn-1", "new", "in_progress", 10, 20);

    assert_eq!(items[0].content, "old");

    items[0]
      .attributes
      .as_mut()
      .expect("attributes")
      .insert("turnId".to_string(), "turn-1".to_string());

    update_streaming_item(&mut items, "turn-1", "new", "completed", 20, 20);

    let attributes = items[0].attributes.as_ref().expect("attributes");
    assert_eq!(items[0].content, "new");
    assert_eq!(attributes.get("modelStatus"), Some(&"ready".to_string()));
    assert_eq!(attributes.get("streamingStatus"), Some(&"completed".to_string()));
  }

  #[test]
  fn streaming_progress_label_caps_at_one_hundred() {
    assert_eq!(streaming_progress_label(250, 100), "100%");
  }
}
