use std::collections::HashMap;

use pith_protocol::TimelineItem;

use super::active_turn_model::ActiveTurn;
use crate::text_utils::take_characters;

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
  attributes
    .entry("responseRole".to_string())
    .or_insert_with(|| "summarizer".to_string());

  items.push(TimelineItem {
    kind: "assistantMessage".to_string(),
    title: "Assistant".to_string(),
    content: initial_content,
    attributes: Some(attributes),
  });

  if is_complete {
    return None;
  }

  Some(ActiveTurn::new(
    turn_id.to_string(),
    thread_id.to_string(),
    full_content,
    initial_chars,
    total_chars,
  ))
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
    .find(|turn| turn.thread_id() == thread_id)
    .map(|turn| turn.id().to_string())
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
    assert_eq!(
      attributes.get("streamingStatus"),
      Some(&"completed".to_string())
    );
  }

  #[test]
  fn streaming_progress_label_caps_at_one_hundred() {
    assert_eq!(streaming_progress_label(250, 100), "100%");
  }

  #[test]
  fn start_streaming_preserves_explicit_response_role() {
    let mut items = vec![];
    let mut attributes = HashMap::new();
    attributes.insert("responseRole".to_string(), "coworkHandoff".to_string());

    let active_turn = start_streaming_assistant_turn(
      "thread-1",
      "turn-1",
      &mut items,
      "short".to_string(),
      attributes,
    );

    assert!(active_turn.is_none());
    assert_eq!(
      items[0]
        .attributes
        .as_ref()
        .and_then(|attributes| attributes.get("responseRole"))
        .map(String::as_str),
      Some("coworkHandoff")
    );
  }
}
