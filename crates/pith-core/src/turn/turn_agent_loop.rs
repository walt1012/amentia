use std::collections::HashMap;

use pith_protocol::TimelineItem;

use super::turn_agent_steps::{AgentStepOutcome, AgentStepRecord};
use crate::request_state::PreparedTurnAction;

const COMPATIBILITY_LOOP_MAX_STEPS: usize = 3;
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
      max_steps: COMPATIBILITY_LOOP_MAX_STEPS,
      turn_id: turn_id.to_string(),
    }
  }

  pub(crate) fn begin_compatibility_step(&self, action: &PreparedTurnAction) -> AgentStepRecord {
    AgentStepRecord::from_turn_action(&self.turn_id, 1, action)
  }

  pub(crate) fn finish_compatibility_step(
    &self,
    step: &AgentStepRecord,
    items: &mut [TimelineItem],
    has_pending_approval: bool,
    has_pending_active_turn: bool,
  ) {
    let outcome = AgentStepOutcome::from_items(
      items,
      has_pending_approval,
      has_pending_active_turn,
    );
    step.tag_items(items, outcome);
    self.tag_loop_items(items);
  }

  fn tag_loop_items(&self, items: &mut [TimelineItem]) {
    for item in items {
      let attributes = item.attributes.get_or_insert_with(HashMap::new);
      attributes.insert("agentLoopId".to_string(), self.loop_id.clone());
      attributes.insert("agentLoopSchema".to_string(), LOOP_SCHEMA.to_string());
      attributes.insert("agentLoopMode".to_string(), "compatibilitySingleAction".to_string());
      attributes.insert("agentLoopMaxSteps".to_string(), self.max_steps.to_string());
      attributes.insert("agentLoopStepCount".to_string(), "1".to_string());
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn compatibility_loop_tags_loop_budget() {
    let coordinator = AgentLoopCoordinator::new("turn-1");
    let action = PreparedTurnAction::ListWorkspace;
    let step = coordinator.begin_compatibility_step(&action);
    let mut items = vec![TimelineItem {
      kind: "plan".to_string(),
      title: "Plan".to_string(),
      content: "List workspace.".to_string(),
      attributes: None,
    }];

    coordinator.finish_compatibility_step(&step, &mut items, false, false);

    let attributes = items[0].attributes.as_ref().expect("attributes");
    assert_eq!(
      attributes.get("agentLoopSchema").map(String::as_str),
      Some("pith.agentLoop.v1")
    );
    assert_eq!(
      attributes.get("agentLoopMode").map(String::as_str),
      Some("compatibilitySingleAction")
    );
    assert_eq!(
      attributes.get("agentLoopMaxSteps").map(String::as_str),
      Some("3")
    );
    assert_eq!(
      attributes.get("agentStepId").map(String::as_str),
      Some("turn-1-step-1")
    );
  }
}
