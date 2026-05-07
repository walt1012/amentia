pub(crate) fn infer_search_query(message: &str) -> Option<String> {
  let trimmed = message.trim();
  let lowercased_message = trimmed.to_lowercase();

  for keyword in ["search for ", "find ", "search "] {
    if let Some(index) = lowercased_message.find(keyword) {
      let query = trimmed[index + keyword.len()..]
        .trim()
        .trim_matches(&['"', '\'', '.', '?', '!', '`'][..]);
      if !query.is_empty() {
        return Some(query.to_string());
      }
    }
  }

  if lowercased_message.contains("grep ") {
    let query = trimmed
      .split_once("grep ")
      .map(|(_, remainder)| remainder.trim())
      .unwrap_or_default()
      .trim_matches(&['"', '\'', '.', '?', '!', '`'][..]);
    if !query.is_empty() {
      return Some(query.to_string());
    }
  }

  None
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn search_query_inference_trims_wrapping_punctuation() {
    assert_eq!(
      infer_search_query("search for `RuntimeContext`?").as_deref(),
      Some("RuntimeContext")
    );
    assert_eq!(
      infer_search_query("grep model_runtime.").as_deref(),
      Some("model_runtime")
    );
  }
}
