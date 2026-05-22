use std::collections::HashMap;

use pith_protocol::TimelineItem;

use super::turn_agent_steps::{AgentStepOutcome, AgentStepRecord};
use crate::request_state::PreparedTurnAction;

const LOOP_MAX_STEPS: usize = 3;
const LOOP_MODE: &str = "dispatcherLoop";
const LOOP_SCHEMA: &str = "pith.agentLoop.v1";

pub(crate) struct AgentLoopCoordinator {
  loop_id: String,
  max_steps: usize,
  turn_id: String,
}

impl AgentLoopCoordinator {
  pub(crate) fn new(turn_id: &str) -> Self {
    Self {
      loop_id: format!("{turn_id}-loop-1"),
      max_steps: LOOP_MAX_STEPS,
      turn_id: turn_id.to_string(),
    }
  }

  pub(crate) fn begin_step(
    &self,
    step_index: usize,
    action: &PreparedTurnAction,
  ) -> AgentStepRecord {
    AgentStepRecord::from_turn_action(&self.turn_id, step_index, action)
  }

  pub(crate) fn finish_step(
    &self,
    step: &AgentStepRecord,
    items: &mut [TimelineItem],
    step_count: usize,
    stop_reason: AgentLoopStopReason,
    has_pending_approval: bool,
    has_pending_active_turn: bool,
  ) {
    let outcome =
      AgentStepOutcome::from_items(items, has_pending_approval, has_pending_active_turn);
    step.tag_items(items, outcome);
    self.tag_loop_items(items, step_count, stop_reason);
  }

  fn tag_loop_items(
    &self,
    items: &mut [TimelineItem],
    step_count: usize,
    stop_reason: AgentLoopStopReason,
  ) {
    let observation_count = count_observations(items);
    for item in items {
      let attributes = item.attributes.get_or_insert_with(HashMap::new);
      attributes.insert("agentLoopId".to_string(), self.loop_id.clone());
      attributes.insert("agentLoopSchema".to_string(), LOOP_SCHEMA.to_string());
      attributes.insert("agentLoopMode".to_string(), LOOP_MODE.to_string());
      attributes.insert("agentLoopMaxSteps".to_string(), self.max_steps.to_string());
      attributes.insert("agentLoopStepCount".to_string(), step_count.to_string());
      attributes.insert(
        "agentLoopBudgetRemaining".to_string(),
        self.max_steps.saturating_sub(step_count).to_string(),
      );
      attributes.insert(
        "agentLoopStopReason".to_string(),
        stop_reason.as_str().to_string(),
      );
      attributes.insert(
        "agentLoopObservationCount".to_string(),
        observation_count.to_string(),
      );
    }
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AgentLoopStopReason {
  ApprovalPaused,
  Cancelled,
  Completed,
  Failed,
  Streaming,
}

impl AgentLoopStopReason {
  pub(crate) fn from_step_state(
    items: &[TimelineItem],
    cancellation_is_cancelled: bool,
    has_pending_approval: bool,
    has_pending_active_turn: bool,
  ) -> Self {
    if cancellation_is_cancelled {
      Self::Cancelled
    } else if has_pending_approval {
      Self::ApprovalPaused
    } else if has_pending_active_turn {
      Self::Streaming
    } else if items.iter().any(is_failure_observation) {
      Self::Failed
    } else {
      Self::Completed
    }
  }

  fn as_str(self) -> &'static str {
    match self {
      Self::ApprovalPaused => "approvalPaused",
      Self::Cancelled => "cancelled",
      Self::Completed => "completed",
      Self::Failed => "failed",
      Self::Streaming => "streaming",
    }
  }
}

fn is_failure_observation(item: &TimelineItem) -> bool {
  if item.kind == "warning" {
    return true;
  }

  item.attributes.as_ref().is_some_and(|attributes| {
    attributes
      .get("pluginCommandStatus")
      .is_some_and(|status| status == "failed")
  })
}

fn count_observations(items: &[TimelineItem]) -> usize {
  items
    .iter()
    .filter(|item| {
      matches!(
        item.kind.as_str(),
        "toolResult" | "pluginResult" | "diffArtifact" | "warning"
      )
    })
    .count()
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn dispatcher_loop_tags_loop_budget_and_stop_reason() {
    let coordinator = AgentLoopCoordinator::new("turn-1");
    let action = PreparedTurnAction::ListWorkspace;
    let step = coordinator.begin_step(1, &action);
    let mut items = vec![TimelineItem {
      kind: "plan".to_string(),
      title: "Plan".to_string(),
      content: "List workspace.".to_string(),
      attributes: None,
    }];

    coordinator.finish_step(
      &step,
      &mut items,
      1,
      AgentLoopStopReason::Completed,
      false,
      false,
    );

    let attributes = items[0].attributes.as_ref().expect("attributes");
    assert_eq!(
      attributes.get("agentLoopSchema").map(String::as_str),
      Some("pith.agentLoop.v1")
    );
    assert_eq!(
      attributes.get("agentLoopMode").map(String::as_str),
      Some("dispatcherLoop")
    );
    assert_eq!(
      attributes.get("agentLoopMaxSteps").map(String::as_str),
      Some("3")
    );
    assert_eq!(
      attributes.get("agentStepId").map(String::as_str),
      Some("turn-1-step-1")
    );
    assert_eq!(
      attributes.get("agentLoopStepCount").map(String::as_str),
      Some("1")
    );
    assert_eq!(
      attributes
        .get("agentLoopBudgetRemaining")
        .map(String::as_str),
      Some("2")
    );
    assert_eq!(
      attributes.get("agentLoopStopReason").map(String::as_str),
      Some("completed")
    );
    assert_eq!(
      attributes
        .get("agentLoopObservationCount")
        .map(String::as_str),
      Some("0")
    );
  }

  #[test]
  fn stop_reason_marks_warning_observations_as_failed() {
    let items = vec![TimelineItem {
      kind: "warning".to_string(),
      title: "Tool Failed".to_string(),
      content: String::new(),
      attributes: None,
    }];

    let reason = AgentLoopStopReason::from_step_state(&items, false, false, false);

    assert_eq!(reason, AgentLoopStopReason::Failed);
  }
}
