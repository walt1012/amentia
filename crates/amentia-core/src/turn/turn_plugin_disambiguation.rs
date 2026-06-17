use amentia_model_runtime::{GenerateRequest, GenerationCancellation, ModelRole};

use super::turn_plugin_routing::{ExplicitPluginCommandRoute, PluginCommandRouteCandidate};
use crate::runtime_context::RuntimeContext;

const CONNECTOR_ROUTE_DECISION_TOKENS: usize = 32;
const CONNECTOR_ROUTE_DISAMBIGUATION_SCORE_WINDOW: usize = 20;

pub(crate) fn maybe_disambiguate_plugin_route(
  context: &RuntimeContext,
  message: &str,
  mut route: ExplicitPluginCommandRoute,
  cancellation: &GenerationCancellation,
) -> ExplicitPluginCommandRoute {
  if cancellation.is_cancelled() {
    route.planning_attributes.insert(
      "toolPlanningDisambiguation".to_string(),
      "cancelled".to_string(),
    );
    return route;
  }

  if !route_needs_model_disambiguation(&route.planning_candidates) {
    route.planning_attributes.insert(
      "toolPlanningDisambiguation".to_string(),
      "notNeeded".to_string(),
    );
    return route;
  }

  route.planning_attributes.insert(
    "toolPlanningDisambiguation".to_string(),
    "requested".to_string(),
  );
  let response = context.model_state.runtime().generate(GenerateRequest {
    role: ModelRole::Planner,
    prompt: connector_disambiguation_prompt(message, &route),
    max_tokens: CONNECTOR_ROUTE_DECISION_TOKENS,
    timeout: None,
    cancellation: Some(cancellation.clone()),
  });
  if response.status != "ready" {
    route.planning_attributes.insert(
      "toolPlanningDisambiguation".to_string(),
      "modelUnavailable".to_string(),
    );
    return route;
  }

  if response.text.trim().eq_ignore_ascii_case("KEEP_CURRENT") {
    route.planning_attributes.insert(
      "toolPlanningDisambiguation".to_string(),
      "modelKeptCurrent".to_string(),
    );
    return route;
  }

  let Some(command_id) =
    selected_connector_candidate_from_model_output(&response.text, &route.planning_candidates)
  else {
    route.planning_attributes.insert(
      "toolPlanningDisambiguation".to_string(),
      "modelUnclear".to_string(),
    );
    return route;
  };

  route
    .planning_attributes
    .insert("toolPlanningModelDecision".to_string(), command_id.clone());
  if command_id == route.command_id {
    route.planning_attributes.insert(
      "toolPlanningDisambiguation".to_string(),
      "modelConfirmed".to_string(),
    );
    return route;
  }

  let initial_command_id = route.command_id.clone();
  route.planning_attributes.insert(
    "toolPlanningInitialCommandId".to_string(),
    initial_command_id,
  );
  route.planning_attributes.insert(
    "toolPlanningSelectedCommandId".to_string(),
    command_id.clone(),
  );
  route.planning_attributes.insert(
    "toolPlanningSelectionState".to_string(),
    "modelDisambiguated".to_string(),
  );
  route.planning_attributes.insert(
    "toolPlanningDisambiguation".to_string(),
    "modelSelected".to_string(),
  );
  route.command_id = command_id;
  route
}

fn route_needs_model_disambiguation(candidates: &[PluginCommandRouteCandidate]) -> bool {
  let Some(selected) = candidates.first() else {
    return false;
  };
  let Some(second) = candidates.get(1) else {
    return false;
  };

  selected.score.saturating_sub(second.score) <= CONNECTOR_ROUTE_DISAMBIGUATION_SCORE_WINDOW
}

fn connector_disambiguation_prompt(message: &str, route: &ExplicitPluginCommandRoute) -> String {
  let candidates = route
    .planning_candidates
    .iter()
    .enumerate()
    .map(|(index, candidate)| {
      format!(
        "{}. command_id: {}\n   title: {}\n   description: {}\n   score: {}",
        index + 1,
        candidate.command_id,
        candidate.title,
        candidate.description,
        candidate.score
      )
    })
    .collect::<Vec<_>>()
    .join("\n");

  format!(
    "Choose the best Amentia connector command for the user request.\n\
     Return exactly one command_id from the candidate list, or KEEP_CURRENT.\n\
     User request: {message}\n\
     Current deterministic command_id: {}\n\
     Candidates:\n{candidates}\n\
     Decision:",
    route.command_id
  )
}

fn selected_connector_candidate_from_model_output(
  output: &str,
  candidates: &[PluginCommandRouteCandidate],
) -> Option<String> {
  let trimmed = output.trim();
  if trimmed.eq_ignore_ascii_case("KEEP_CURRENT") {
    return None;
  }

  let mut matches = candidates
    .iter()
    .filter(|candidate| trimmed.contains(&candidate.command_id));
  let selected = matches.next()?;
  if matches.next().is_some() {
    return None;
  }
  Some(selected.command_id.clone())
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn disambiguation_only_runs_for_close_connector_scores() {
    let far = vec![
      candidate("notion-connector::notion.prepare-page-draft", 120),
      candidate("linear-connector::linear.prepare-issue-draft", 80),
    ];
    let close = vec![
      candidate("notion-connector::notion.prepare-page-draft", 120),
      candidate("linear-connector::linear.prepare-issue-draft", 110),
    ];

    assert!(!route_needs_model_disambiguation(&far));
    assert!(route_needs_model_disambiguation(&close));
  }

  #[test]
  fn model_output_must_match_a_single_candidate_command_id() {
    let candidates = vec![
      candidate("notion-connector::notion.prepare-page-draft", 120),
      candidate("linear-connector::linear.prepare-issue-draft", 110),
    ];

    assert_eq!(
      selected_connector_candidate_from_model_output(
        "linear-connector::linear.prepare-issue-draft",
        &candidates
      )
      .as_deref(),
      Some("linear-connector::linear.prepare-issue-draft")
    );
    assert_eq!(
      selected_connector_candidate_from_model_output("use the issue tool", &candidates),
      None
    );
    assert_eq!(
      selected_connector_candidate_from_model_output(
        "notion-connector::notion.prepare-page-draft or linear-connector::linear.prepare-issue-draft",
        &candidates
      ),
      None
    );
  }

  fn candidate(command_id: &str, score: usize) -> PluginCommandRouteCandidate {
    PluginCommandRouteCandidate {
      command_id: command_id.to_string(),
      title: command_id.to_string(),
      description: "Connector command".to_string(),
      score,
    }
  }
}
