pub fn take_characters(content: &str, count: usize) -> String {
  content.chars().take(count).collect()
}

pub fn take_last_characters(content: &str, count: usize) -> String {
  let character_count = content.chars().count();
  if character_count <= count {
    return content.to_string();
  }

  content
    .chars()
    .skip(character_count.saturating_sub(count))
    .collect()
}

pub fn truncate_text(content: &str, limit: usize) -> String {
  let normalized = content.split_whitespace().collect::<Vec<_>>().join(" ");
  if normalized.chars().count() <= limit {
    return normalized;
  }

  let truncated = normalized
    .chars()
    .take(limit.saturating_sub(3))
    .collect::<String>();
  format!("{truncated}...")
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn truncate_text_normalizes_whitespace() {
    assert_eq!(truncate_text("one\n  two\tthree", 100), "one two three");
  }

  #[test]
  fn truncate_text_preserves_unicode_boundaries() {
    assert_eq!(truncate_text("世界abcdef", 5), "世界...");
  }

  #[test]
  fn take_last_characters_returns_tail() {
    assert_eq!(take_last_characters("abcdef", 3), "def");
  }
}
