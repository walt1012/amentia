const NOTION_PAGE_DRAFT_COMMAND_ID: &str = "notion-connector::notion.prepare-page-draft";
const NATURAL_NOTION_DRAFT_REASON: &str = "naturalNotionDraftCommand";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExplicitPluginCommandRoute {
  pub(crate) command_id: String,
  pub(crate) input: Option<String>,
  pub(crate) routing_reason: &'static str,
}

pub(crate) fn infer_explicit_plugin_command_route(
  message: &str,
) -> Option<ExplicitPluginCommandRoute> {
  let trimmed = message.trim();
  for (prefix, routing_reason) in explicit_plugin_prefixes() {
    let Some(remainder) = strip_case_insensitive_prefix(trimmed, prefix) else {
      continue;
    };
    let Some((command_id, input)) = split_command_route(remainder) else {
      continue;
    };
    if !is_supported_command_id(command_id) {
      continue;
    }

    return Some(ExplicitPluginCommandRoute {
      command_id: command_id.to_string(),
      input: input.map(str::to_string),
      routing_reason,
    });
  }

  None
}

pub(crate) fn infer_natural_plugin_command_route(
  message: &str,
) -> Option<ExplicitPluginCommandRoute> {
  let normalized = message.to_ascii_lowercase();
  if !normalized.contains("notion") || !looks_like_notion_draft_request(&normalized) {
    return None;
  }

  Some(ExplicitPluginCommandRoute {
    command_id: NOTION_PAGE_DRAFT_COMMAND_ID.to_string(),
    input: Some(message.trim().to_string()).filter(|input| !input.is_empty()),
    routing_reason: NATURAL_NOTION_DRAFT_REASON,
  })
}

fn explicit_plugin_prefixes() -> [(&'static str, &'static str); 5] {
  [
    ("/plugin ", "slashPluginCommand"),
    ("run plugin command ", "explicitPluginCommand"),
    ("run plugin ", "explicitPluginCommand"),
    ("use plugin command ", "explicitPluginCommand"),
    ("use plugin ", "explicitPluginCommand"),
  ]
}

fn looks_like_notion_draft_request(normalized: &str) -> bool {
  let action_match = [
    "prepare", "draft", "create", "make", "compose", "summarize", "summary",
  ]
  .iter()
  .any(|term| normalized.contains(term));
  let artifact_match = ["page", "note", "handoff", "brief", "update"]
    .iter()
    .any(|term| normalized.contains(term));

  action_match && artifact_match
}

fn strip_case_insensitive_prefix<'a>(value: &'a str, prefix: &str) -> Option<&'a str> {
  value
    .get(..prefix.len())
    .filter(|candidate| candidate.eq_ignore_ascii_case(prefix))
    .map(|_| value[prefix.len()..].trim())
}

fn split_command_route(remainder: &str) -> Option<(&str, Option<&str>)> {
  let trimmed = remainder.trim();
  if trimmed.is_empty() {
    return None;
  }

  let split_at = trimmed.find(char::is_whitespace).unwrap_or(trimmed.len());
  let command_id = &trimmed[..split_at];
  let input = trimmed[split_at..].trim();
  Some((command_id, (!input.is_empty()).then_some(input)))
}

fn is_supported_command_id(command_id: &str) -> bool {
  command_id.contains("::")
    && !command_id.starts_with("::")
    && !command_id.ends_with("::")
    && command_id.chars().all(|character| {
      character.is_ascii_alphanumeric() || matches!(character, ':' | '.' | '_' | '-')
    })
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn detects_slash_plugin_command_with_input() {
    let route =
      infer_explicit_plugin_command_route("/plugin notion-runner::notion-runner.sync sync today")
        .expect("route");

    assert_eq!(route.command_id, "notion-runner::notion-runner.sync");
    assert_eq!(route.input.as_deref(), Some("sync today"));
    assert_eq!(route.routing_reason, "slashPluginCommand");
  }

  #[test]
  fn detects_case_insensitive_run_plugin_command() {
    let route = infer_explicit_plugin_command_route(
      "Run Plugin Command workspace-notes::workspace.capture-note",
    )
    .expect("route");

    assert_eq!(route.command_id, "workspace-notes::workspace.capture-note");
    assert_eq!(route.input, None);
    assert_eq!(route.routing_reason, "explicitPluginCommand");
  }

  #[test]
  fn ignores_natural_language_plugin_mentions() {
    let route = infer_explicit_plugin_command_route("Can a notion plugin help this thread?");

    assert!(route.is_none());
  }

  #[test]
  fn detects_natural_notion_page_draft_request() {
    let route = infer_natural_plugin_command_route(
      "Prepare a Notion page draft for this project handoff.",
    )
    .expect("route");

    assert_eq!(route.command_id, NOTION_PAGE_DRAFT_COMMAND_ID);
    assert_eq!(
      route.input.as_deref(),
      Some("Prepare a Notion page draft for this project handoff.")
    );
    assert_eq!(route.routing_reason, "naturalNotionDraftCommand");
  }

  #[test]
  fn ignores_natural_notion_lookup_request() {
    let route = infer_natural_plugin_command_route("What is new in Notion?");

    assert!(route.is_none());
  }

  #[test]
  fn ignores_invalid_command_ids() {
    assert!(infer_explicit_plugin_command_route("/plugin missing").is_none());
    assert!(infer_explicit_plugin_command_route("/plugin bad command").is_none());
  }
}
