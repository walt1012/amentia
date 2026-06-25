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
    "browse the web for ",
    "check online for ",
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

  if let Some(index) = lowercased_message.find("look up ") {
    let query = trimmed[index + "look up ".len()..]
      .trim()
      .trim_matches(&['"', '\'', '.', '?', '!', '`'][..]);
    if !query.is_empty() && !has_local_workspace_signal(&query.to_lowercase()) {
      return Some(WebSearchIntent {
        query: query.to_string(),
        routing_reason: "explicitWebSearchRequest",
      });
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

pub(crate) fn infer_model_web_search_intent(message: &str) -> Option<WebSearchIntent> {
  let trimmed = message
    .trim()
    .trim_matches(&['"', '\'', '.', '?', '!', '`'][..]);
  if trimmed.is_empty() {
    return None;
  }

  let lowercased_message = trimmed.to_lowercase();
  if has_local_workspace_signal(&lowercased_message) {
    return None;
  }

  Some(WebSearchIntent {
    query: trimmed.to_string(),
    routing_reason: "modelToolPlanning",
  })
}

fn has_fresh_public_information_signal(message: &str) -> bool {
  [
    "latest",
    "current",
    "today",
    "right now",
    "as of",
    "up to date",
    "up-to-date",
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
      infer_explicit_web_search_intent("web search for `Amentia local model`?").expect("intent");
    assert_eq!(local_model.query, "Amentia local model");

    let granite = infer_explicit_web_search_intent("search the web for IBM Granite 4.0-H-350M")
      .expect("intent");
    assert_eq!(granite.query, "IBM Granite 4.0-H-350M");

    let plugins = infer_explicit_web_search_intent("websearch amentia plugins").expect("intent");
    assert_eq!(plugins.query, "amentia plugins");
    let browser = infer_explicit_web_search_intent("browse the web for Amentia").expect("intent");
    assert_eq!(browser.query, "Amentia");
    let online =
      infer_explicit_web_search_intent("check online for Granite 4.0-H-350M").expect("intent");
    assert_eq!(online.query, "Granite 4.0-H-350M");
    let lookup = infer_explicit_web_search_intent("look up IBM Granite").expect("lookup intent");
    assert_eq!(lookup.query, "IBM Granite");
    assert!(infer_explicit_web_search_intent("look up README.md").is_none());
    assert!(infer_explicit_web_search_intent("search RuntimeContext").is_none());
  }

  #[test]
  fn fresh_web_search_query_inference_uses_current_information_signals() {
    let release = infer_fresh_web_search_intent("What is the latest Granite 4.0-H-350M release?")
      .expect("intent");
    assert_eq!(
      release.query,
      "What is the latest Granite 4.0-H-350M release"
    );
    let ceo = infer_fresh_web_search_intent("Who is the CEO of IBM?").expect("intent");
    assert_eq!(ceo.routing_reason, "freshPublicInformation");
    let stock =
      infer_fresh_web_search_intent("What is Apple's stock price today?").expect("intent");
    assert_eq!(stock.query, "What is Apple's stock price today");
    let current = infer_fresh_web_search_intent("Is the Granite 4.0-H-350M model list up to date?")
      .expect("intent");
    assert_eq!(current.routing_reason, "freshPublicInformation");
    assert!(infer_fresh_web_search_intent("What changed in this repo?").is_none());
    assert!(infer_fresh_web_search_intent("What version is in Cargo.toml?").is_none());
    assert!(infer_fresh_web_search_intent("How should I proceed now?").is_none());
  }

  #[test]
  fn model_web_search_candidate_keeps_external_questions_available_to_planner() {
    let comparison = infer_model_web_search_intent("Compare Codex and Claude Code plugin systems")
      .expect("candidate");
    assert_eq!(comparison.routing_reason, "modelToolPlanning");
    assert_eq!(
      comparison.query,
      "Compare Codex and Claude Code plugin systems"
    );
    assert!(infer_model_web_search_intent("Explain this repo architecture").is_none());
  }

  #[test]
  fn web_search_intent_exposes_routing_reason() {
    let explicit =
      infer_explicit_web_search_intent("search the web for Amentia").expect("explicit intent");
    assert_eq!(explicit.routing_reason, "explicitWebSearchRequest");

    let fresh =
      infer_fresh_web_search_intent("What is the current Rust release?").expect("fresh intent");
    assert_eq!(fresh.routing_reason, "freshPublicInformation");
  }
}
