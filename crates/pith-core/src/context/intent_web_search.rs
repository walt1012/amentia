pub(crate) fn infer_explicit_web_search_query(message: &str) -> Option<String> {
  let trimmed = message.trim();
  let lowercased_message = trimmed.to_lowercase();

  for keyword in [
    "websearch for ",
    "websearch ",
    "web search for ",
    "web search ",
    "internet search for ",
    "search internet for ",
    "search web for ",
    "search the web for ",
    "search online for ",
    "look up online ",
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

pub(crate) fn infer_fresh_web_search_query(message: &str) -> Option<String> {
  let trimmed = message
    .trim()
    .trim_matches(&['"', '\'', '.', '?', '!', '`'][..]);
  if trimmed.is_empty() {
    return None;
  }

  let lowercased_message = trimmed.to_lowercase();
  let has_freshness_signal = [
    "latest",
    "current",
    "today",
    "now",
    "recent",
    "news",
    "release",
    "released",
    "version",
    "price",
    "schedule",
    "score",
    "weather",
    "who is",
    "what is the newest",
    "what is the current",
  ]
  .iter()
  .any(|signal| lowercased_message.contains(signal));
  let has_local_signal = [
    "workspace",
    "repo",
    "repository",
    "file",
    "directory",
    "folder",
    "readme",
    "diff",
    "commit",
    "branch",
  ]
  .iter()
  .any(|signal| lowercased_message.contains(signal));

  if has_freshness_signal && !has_local_signal {
    Some(trimmed.to_string())
  } else {
    None
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn web_search_query_inference_requires_web_intent() {
    assert_eq!(
      infer_explicit_web_search_query("web search for `Pith local model`?").as_deref(),
      Some("Pith local model")
    );
    assert_eq!(
      infer_explicit_web_search_query("search the web for Liquid AI LFM2.5").as_deref(),
      Some("Liquid AI LFM2.5")
    );
    assert_eq!(
      infer_explicit_web_search_query("websearch pith plugins").as_deref(),
      Some("pith plugins")
    );
    assert_eq!(infer_explicit_web_search_query("look up README.md"), None);
    assert_eq!(
      infer_explicit_web_search_query("search RuntimeContext"),
      None
    );
  }

  #[test]
  fn fresh_web_search_query_inference_uses_current_information_signals() {
    assert_eq!(
      infer_fresh_web_search_query("What is the latest LFM2.5 release?").as_deref(),
      Some("What is the latest LFM2.5 release")
    );
    assert_eq!(
      infer_fresh_web_search_query("What changed in this repo?"),
      None
    );
  }
}
