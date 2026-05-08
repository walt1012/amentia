pub(crate) fn infer_web_search_query(message: &str) -> Option<String> {
  let trimmed = message.trim();
  let lowercased_message = trimmed.to_lowercase();

  for keyword in [
    "web search for ",
    "search web for ",
    "search the web for ",
    "search online for ",
    "look up online ",
    "look up ",
  ] {
    if let Some(index) = lowercased_message.find(keyword) {
      let query = trimmed[index + keyword.len()..]
        .trim()
        .trim_matches(&['"', '\'', '.', '?', '!', '`'][..]);
      if !query.is_empty() {
        return Some(query.to_string());
      }
    }
  }

  None
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn web_search_query_inference_requires_web_intent() {
    assert_eq!(
      infer_web_search_query("web search for `Pith local model`?").as_deref(),
      Some("Pith local model")
    );
    assert_eq!(
      infer_web_search_query("search the web for Liquid AI LFM2.5").as_deref(),
      Some("Liquid AI LFM2.5")
    );
    assert_eq!(infer_web_search_query("search RuntimeContext"), None);
  }
}
