use std::collections::HashSet;

pub(crate) fn token_set(content: &str) -> HashSet<String> {
  normalize_text(content)
    .split_whitespace()
    .filter(|token| !token.is_empty())
    .map(ToOwned::to_owned)
    .collect()
}

pub(crate) fn normalize_text(content: &str) -> String {
  content
    .chars()
    .map(|character| {
      if character.is_ascii_alphanumeric() {
        character.to_ascii_lowercase()
      } else {
        ' '
      }
    })
    .collect()
}
