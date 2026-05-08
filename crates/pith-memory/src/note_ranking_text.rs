use std::collections::HashSet;

pub(crate) fn normalize_text(value: &str) -> String {
  value
    .chars()
    .map(|character| {
      if character.is_alphanumeric() {
        character.to_ascii_lowercase()
      } else {
        ' '
      }
    })
    .collect::<String>()
    .split_whitespace()
    .collect::<Vec<_>>()
    .join(" ")
}

pub(crate) fn token_set(value: &str) -> HashSet<String> {
  normalize_text(value)
    .split_whitespace()
    .filter(|token| token.len() > 2)
    .map(str::to_string)
    .collect()
}
