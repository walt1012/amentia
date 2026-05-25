use std::collections::HashMap;

use pith_plugin_host::{
  build_command_registry, build_connector_registry, PluginCatalogEntry,
  PluginCommandEntry as HostPluginCommandEntry, PluginConnectorEntry as HostPluginConnectorEntry,
};

use crate::plugins::plugin_command_artifacts::connector_command_input;

const NOTION_PAGE_DRAFT_COMMAND_ID: &str = "notion-connector::notion.prepare-page-draft";
const REVIEW_DIFF_COMMAND_ID: &str = "review-assistant::review.inspect-diff";
const WORKSPACE_NOTE_COMMAND_ID: &str = "workspace-notes::workspace.capture-note";
const NATURAL_CONNECTOR_COMMAND_REASON: &str = "naturalConnectorCommand";
const NATURAL_NOTION_DRAFT_REASON: &str = "naturalNotionDraftCommand";
const NATURAL_REVIEW_DIFF_REASON: &str = "naturalReviewDiffCommand";
const NATURAL_WORKSPACE_NOTE_REASON: &str = "naturalWorkspaceNoteCommand";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExplicitPluginCommandRoute {
  pub(crate) command_id: String,
  pub(crate) input: Option<String>,
  pub(crate) routing_reason: &'static str,
  pub(crate) planning_attributes: HashMap<String, String>,
  pub(crate) planning_candidates: Vec<PluginCommandRouteCandidate>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PluginCommandRouteCandidate {
  pub(crate) command_id: String,
  pub(crate) title: String,
  pub(crate) description: String,
  pub(crate) score: usize,
}

struct ConnectorCommandCandidate<'a> {
  score: usize,
  connector_score: usize,
  action_score: usize,
  command: &'a HostPluginCommandEntry,
}

struct ConnectorPlanningScores {
  selected_score: usize,
  selected_connector_score: usize,
  selected_action_score: usize,
  second_score: Option<usize>,
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
      planning_attributes: route_planning_attributes(
        "explicitCommand",
        routing_reason,
        command_id,
        1,
        None,
      ),
      planning_candidates: Vec::new(),
    });
  }

  None
}

pub(crate) fn infer_natural_plugin_command_route(
  message: &str,
  plugins: &[PluginCatalogEntry],
) -> Option<ExplicitPluginCommandRoute> {
  infer_natural_connector_command_route(message, plugins)
    .or_else(|| infer_natural_builtin_plugin_command_route(message))
}

fn infer_natural_builtin_plugin_command_route(message: &str) -> Option<ExplicitPluginCommandRoute> {
  let trimmed = message.trim();
  let normalized = trimmed.to_ascii_lowercase();
  let (command_id, routing_reason) =
    if normalized.contains("notion") && looks_like_notion_draft_request(&normalized) {
      (NOTION_PAGE_DRAFT_COMMAND_ID, NATURAL_NOTION_DRAFT_REASON)
    } else if looks_like_workspace_note_capture_request(&normalized) {
      (WORKSPACE_NOTE_COMMAND_ID, NATURAL_WORKSPACE_NOTE_REASON)
    } else if looks_like_review_diff_request(&normalized) {
      (REVIEW_DIFF_COMMAND_ID, NATURAL_REVIEW_DIFF_REASON)
    } else {
      return None;
    };

  Some(ExplicitPluginCommandRoute {
    command_id: command_id.to_string(),
    input: Some(trimmed.to_string()).filter(|input| !input.is_empty()),
    routing_reason,
    planning_attributes: route_planning_attributes(
      "builtinNaturalCommand",
      routing_reason,
      command_id,
      1,
      None,
    ),
    planning_candidates: Vec::new(),
  })
}

fn infer_natural_connector_command_route(
  message: &str,
  plugins: &[PluginCatalogEntry],
) -> Option<ExplicitPluginCommandRoute> {
  let trimmed = message.trim();
  if trimmed.is_empty() {
    return None;
  }
  let normalized = normalize_match_text(trimmed);
  let connectors = build_connector_registry(plugins);
  if connectors.is_empty() {
    return None;
  }
  let commands = build_command_registry(plugins);

  let mut candidates = commands
    .iter()
    .filter_map(|command| {
      let scoped_connectors = command_scoped_connectors(command, &connectors);
      if scoped_connectors.is_empty() {
        return None;
      }
      let connector_score = scoped_connectors
        .iter()
        .map(|connector| connector_match_score(&normalized, connector))
        .max()
        .unwrap_or(0);
      if connector_score == 0 {
        return None;
      }
      let action_score = command_action_score(&normalized, command);
      if action_score == 0 && !looks_like_connector_action_request(&normalized) {
        return None;
      }

      Some(ConnectorCommandCandidate {
        score: connector_score + action_score,
        connector_score,
        action_score,
        command,
      })
    })
    .collect::<Vec<_>>();
  candidates.sort_by(|left, right| {
    right
      .score
      .cmp(&left.score)
      .then_with(|| left.command.title.cmp(&right.command.title))
      .then_with(|| left.command.command_id.cmp(&right.command.command_id))
  });
  let selected = candidates.first()?;
  let second_score = candidates.get(1).map(|candidate| candidate.score);

  Some(ExplicitPluginCommandRoute {
    command_id: selected.command.command_id.clone(),
    input: connector_command_input(trimmed),
    routing_reason: NATURAL_CONNECTOR_COMMAND_REASON,
    planning_attributes: route_planning_attributes(
      "deterministicConnectorRanking",
      NATURAL_CONNECTOR_COMMAND_REASON,
      &selected.command.command_id,
      candidates.len(),
      Some(ConnectorPlanningScores {
        selected_score: selected.score,
        selected_connector_score: selected.connector_score,
        selected_action_score: selected.action_score,
        second_score,
      }),
    ),
    planning_candidates: connector_route_candidates(&candidates),
  })
}

fn connector_route_candidates(
  candidates: &[ConnectorCommandCandidate<'_>],
) -> Vec<PluginCommandRouteCandidate> {
  candidates
    .iter()
    .take(4)
    .map(|candidate| PluginCommandRouteCandidate {
      command_id: candidate.command.command_id.clone(),
      title: candidate.command.title.clone(),
      description: candidate.command.description.clone(),
      score: candidate.score,
    })
    .collect()
}

fn route_planning_attributes(
  mode: &str,
  reason: &str,
  command_id: &str,
  candidate_count: usize,
  scores: Option<ConnectorPlanningScores>,
) -> HashMap<String, String> {
  let mut attributes = HashMap::from([
    ("toolPlanningMode".to_string(), mode.to_string()),
    ("toolPlanningReason".to_string(), reason.to_string()),
    (
      "toolPlanningSelectedCommandId".to_string(),
      command_id.to_string(),
    ),
    (
      "toolPlanningCandidateCount".to_string(),
      candidate_count.to_string(),
    ),
  ]);
  let Some(scores) = scores else {
    attributes.insert(
      "toolPlanningSelectionState".to_string(),
      "direct".to_string(),
    );
    return attributes;
  };

  attributes.insert(
    "toolPlanningSelectedScore".to_string(),
    scores.selected_score.to_string(),
  );
  attributes.insert(
    "toolPlanningSelectedConnectorScore".to_string(),
    scores.selected_connector_score.to_string(),
  );
  attributes.insert(
    "toolPlanningSelectedActionScore".to_string(),
    scores.selected_action_score.to_string(),
  );
  let selection_state = match scores.second_score {
    Some(second_score) if second_score == scores.selected_score => {
      attributes.insert(
        "toolPlanningSecondScore".to_string(),
        second_score.to_string(),
      );
      "deterministicTieBreak"
    }
    Some(second_score) => {
      attributes.insert(
        "toolPlanningSecondScore".to_string(),
        second_score.to_string(),
      );
      "deterministicRanked"
    }
    None => "deterministicSingle",
  };
  attributes.insert(
    "toolPlanningSelectionState".to_string(),
    selection_state.to_string(),
  );

  attributes
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
    "prepare",
    "draft",
    "create",
    "make",
    "compose",
    "summarize",
    "summary",
  ]
  .iter()
  .any(|term| normalized.contains(term));
  let artifact_match = ["page", "note", "handoff", "brief", "update"]
    .iter()
    .any(|term| normalized.contains(term));

  action_match && artifact_match
}

fn command_scoped_connectors<'a>(
  command: &HostPluginCommandEntry,
  connectors: &'a [HostPluginConnectorEntry],
) -> Vec<&'a HostPluginConnectorEntry> {
  let plugin_connectors = connectors
    .iter()
    .filter(|connector| connector.plugin_id == command.plugin_id)
    .collect::<Vec<_>>();
  let Some(connector_ids) = command
    .execution
    .as_ref()
    .and_then(|execution| execution.connector_ids.as_ref())
  else {
    return plugin_connectors;
  };
  if connector_ids.is_empty() {
    return vec![];
  }

  plugin_connectors
    .into_iter()
    .filter(|connector| {
      connector_ids.iter().any(|connector_id| {
        let qualified = qualified_connector_id(&command.plugin_id, connector_id);
        connector.connector_id.as_str() == connector_id.as_str()
          || connector.connector_id.as_str() == qualified.as_str()
      })
    })
    .collect()
}

fn connector_match_score(normalized_message: &str, connector: &HostPluginConnectorEntry) -> usize {
  let mut score = 0;
  for term in [
    connector.service.as_str(),
    connector.display_name.as_str(),
    connector.plugin_display_name.as_str(),
  ] {
    if normalized_contains_term(normalized_message, term) {
      score += 100;
    }
  }
  score
}

fn command_action_score(normalized_message: &str, command: &HostPluginCommandEntry) -> usize {
  let command_text = normalize_match_text(&format!(
    "{} {} {}",
    command.title, command.description, command.prompt
  ));
  action_terms()
    .iter()
    .map(|term| {
      if !normalized_message.contains(*term) {
        0
      } else if command_text.contains(*term) {
        20
      } else if command_looks_actionable(&command_text) {
        10
      } else {
        0
      }
    })
    .sum()
}

fn looks_like_connector_action_request(normalized: &str) -> bool {
  action_terms().iter().any(|term| normalized.contains(*term))
}

fn command_looks_actionable(normalized_command_text: &str) -> bool {
  action_terms()
    .iter()
    .any(|term| normalized_command_text.contains(*term))
}

fn action_terms() -> &'static [&'static str] {
  &[
    "brief",
    "capture",
    "compose",
    "create",
    "draft",
    "find",
    "handoff",
    "list",
    "make",
    "prepare",
    "publish",
    "query",
    "record",
    "review",
    "save",
    "search",
    "send",
    "post",
    "summarize",
    "summary",
    "sync",
    "update",
    "write",
  ]
}

fn normalized_contains_term(normalized_message: &str, term: &str) -> bool {
  let normalized_term = normalize_match_text(term);
  normalized_term.len() >= 3 && normalized_message.contains(&normalized_term)
}

fn normalize_match_text(value: &str) -> String {
  value
    .chars()
    .map(|character| {
      if character.is_ascii_alphanumeric() {
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

fn qualified_connector_id(plugin_id: &str, connector_id: &str) -> String {
  if connector_id.contains("::") {
    connector_id.to_string()
  } else {
    format!("{plugin_id}::{connector_id}")
  }
}

fn looks_like_workspace_note_capture_request(normalized: &str) -> bool {
  let action_match = ["capture", "remember", "save", "store", "record"]
    .iter()
    .any(|term| normalized.contains(term));
  let note_match = ["note", "memory", "preference", "context"]
    .iter()
    .any(|term| normalized.contains(term));
  let scope_match = ["workspace", "project", "repo", "repository"]
    .iter()
    .any(|term| normalized.contains(term));

  action_match && note_match && scope_match
}

fn looks_like_review_diff_request(normalized: &str) -> bool {
  let action_match = ["review", "inspect", "check", "summarize"]
    .iter()
    .any(|term| normalized.contains(term));
  let diff_match = normalized.contains("diff")
    || normalized.contains("git changes")
    || normalized.contains("local changes")
    || normalized.contains("uncommitted");

  action_match && diff_match
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
    let route = infer_natural_builtin_plugin_command_route(
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
  fn detects_natural_workspace_note_request() {
    let route =
      infer_natural_builtin_plugin_command_route("Capture a workspace note for this project.")
        .expect("route");

    assert_eq!(route.command_id, WORKSPACE_NOTE_COMMAND_ID);
    assert_eq!(
      route.input.as_deref(),
      Some("Capture a workspace note for this project.")
    );
    assert_eq!(route.routing_reason, "naturalWorkspaceNoteCommand");
  }

  #[test]
  fn detects_natural_review_diff_request() {
    let route =
      infer_natural_builtin_plugin_command_route("Review the current git diff.").expect("route");

    assert_eq!(route.command_id, REVIEW_DIFF_COMMAND_ID);
    assert_eq!(route.input.as_deref(), Some("Review the current git diff."));
    assert_eq!(route.routing_reason, "naturalReviewDiffCommand");
  }

  #[test]
  fn ignores_natural_notion_lookup_request() {
    let route = infer_natural_builtin_plugin_command_route("What is new in Notion?");

    assert!(route.is_none());
  }

  #[test]
  fn ignores_invalid_command_ids() {
    assert!(infer_explicit_plugin_command_route("/plugin missing").is_none());
    assert!(infer_explicit_plugin_command_route("/plugin bad command").is_none());
  }
}
