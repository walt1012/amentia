#[derive(Debug, Clone)]
pub(crate) struct WebSearchIntent {
  pub(crate) query: String,
  pub(crate) routing_reason: &'static str,
}

pub(crate) fn infer_explicit_web_search_intent(message: &str) -> Option<WebSearchIntent> {
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
        return Some(WebSearchIntent {
          query: query.to_string(),
          routing_reason: "explicitWebSearchRequest",
        });
      }
    }
  }

  None
}

pub(crate) fn infer_fresh_web_search_intent(message: &str) -> Option<WebSearchIntent> {
  let trimmed = message
    .trim()
    .trim_matches(&['"', '\'', '.', '?', '!', '`'][..]);
  if trimmed.is_empty() {
    return None;
  }

  let lowercased_message = trimmed.to_lowercase();
  let has_freshness_signal = has_fresh_public_information_signal(&lowercased_message);
  let has_local_signal = has_local_workspace_signal(&lowercased_message);

  if has_freshness_signal && !has_local_signal {
    Some(WebSearchIntent {
      query: trimmed.to_string(),
      routing_reason: "freshPublicInformation",
    })
  } else {
    None
  }
}

fn has_fresh_public_information_signal(message: &str) -> bool {
  [
    "latest",
    "current",
    "today",
    "right now",
    "as of",
    "this week",
    "this month",
    "recent",
    "news",
    "release",
    "released",
    "version",
    "price",
    "stock",
    "exchange rate",
    "schedule",
    "score",
    "weather",
    "ceo",
    "president",
    "who is",
    "what is the newest",
    "what is the current",
    "newest",
  ]
  .iter()
  .any(|signal| message.contains(signal))
}

fn has_local_workspace_signal(message: &str) -> bool {
  [
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
    "project",
    "codebase",
    "cargo.toml",
    "package.json",
    "package.swift",
    "pyproject.toml",
  ]
  .iter()
  .any(|signal| message.contains(signal))
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn web_search_query_inference_requires_web_intent() {
    let local_model =
      infer_explicit_web_search_intent("web search for `Pith local model`?").expect("intent");
    assert_eq!(local_model.query, "Pith local model");

    let lfm =
      infer_explicit_web_search_intent("search the web for Liquid AI LFM2.5").expect("intent");
    assert_eq!(lfm.query, "Liquid AI LFM2.5");

    let plugins = infer_explicit_web_search_intent("websearch pith plugins").expect("intent");
    assert_eq!(plugins.query, "pith plugins");
    assert!(infer_explicit_web_search_intent("look up README.md").is_none());
    assert!(infer_explicit_web_search_intent("search RuntimeContext").is_none());
  }

  #[test]
  fn fresh_web_search_query_inference_uses_current_information_signals() {
    let release =
      infer_fresh_web_search_intent("What is the latest LFM2.5 release?").expect("intent");
    assert_eq!(release.query, "What is the latest LFM2.5 release");
    let ceo = infer_fresh_web_search_intent("Who is the CEO of Liquid AI?").expect("intent");
    assert_eq!(ceo.routing_reason, "freshPublicInformation");
    let stock =
      infer_fresh_web_search_intent("What is Apple's stock price today?").expect("intent");
    assert_eq!(stock.query, "What is Apple's stock price today");
    assert!(infer_fresh_web_search_intent("What changed in this repo?").is_none());
    assert!(infer_fresh_web_search_intent("What version is in Cargo.toml?").is_none());
    assert!(infer_fresh_web_search_intent("How should I proceed now?").is_none());
  }

  #[test]
  fn web_search_intent_exposes_routing_reason() {
    let explicit =
      infer_explicit_web_search_intent("search the web for Pith").expect("explicit intent");
    assert_eq!(explicit.routing_reason, "explicitWebSearchRequest");

    let fresh =
      infer_fresh_web_search_intent("What is the current Rust release?").expect("fresh intent");
    assert_eq!(fresh.routing_reason, "freshPublicInformation");
  }
}
