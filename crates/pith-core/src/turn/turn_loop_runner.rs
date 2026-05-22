use pith_protocol::TimelineItem;

use super::turn_agent_loop::{AgentLoopCoordinator, AgentLoopObservation, AgentLoopStopReason};
use super::turn_step_dispatcher::{TurnStepControl, TurnStepDispatcher, TurnStepResult};
use crate::active_turns::ActiveTurn;
use crate::approval_types::PendingApproval;
use crate::plugin_commands::PluginCommandOutput;
use crate::request_state::{PreparedTurnAction, PreparedTurnSnapshot};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TurnLoopRunSummary {
  pub(crate) step_count: usize,
  pub(crate) stop_reason: AgentLoopStopReason,
}

pub(crate) struct TurnLoopRunner<'a> {
  snapshot: &'a PreparedTurnSnapshot,
  coordinator: AgentLoopCoordinator,
  items: &'a mut Vec<TimelineItem>,
  pending_active_turn: &'a mut Option<ActiveTurn>,
  pending_approval: &'a mut Option<PendingApproval>,
  plugin_command_output: &'a mut Option<PluginCommandOutput>,
}

impl<'a> TurnLoopRunner<'a> {
  pub(crate) fn new(
    snapshot: &'a PreparedTurnSnapshot,
    items: &'a mut Vec<TimelineItem>,
    pending_active_turn: &'a mut Option<ActiveTurn>,
    pending_approval: &'a mut Option<PendingApproval>,
    plugin_command_output: &'a mut Option<PluginCommandOutput>,
  ) -> Self {
    Self {
      snapshot,
      coordinator: AgentLoopCoordinator::new(&snapshot.turn_id),
      items,
      pending_active_turn,
      pending_approval,
      plugin_command_output,
    }
  }

  pub(crate) fn run(&mut self, initial_action: PreparedTurnAction) -> TurnLoopRunSummary {
    let mut next_action = Some(initial_action);
    let mut step_count = 0;
    let mut final_stop_reason = None;

    while let Some(action) = next_action.take() {
      if step_count >= self.coordinator.max_steps() {
        final_stop_reason = Some(AgentLoopStopReason::StepBudgetExhausted);
        break;
      }

      step_count += 1;
      let step_start_index = self.items.len();
      let agent_step = self.coordinator.begin_step(step_count, &action);
      let step_result = self.dispatch_action(action);
      let observation = AgentLoopObservation::from_items(&self.items[step_start_index..]);
      let stop_reason = AgentLoopStopReason::from_step_state(
        observation,
        self.snapshot.cancellation.is_cancelled(),
        self.pending_approval.is_some(),
        self.pending_active_turn.is_some(),
      );
      self.coordinator.finish_step(
        &agent_step,
        &mut self.items[step_start_index..],
        observation,
        step_count,
        stop_reason,
        self.pending_approval.is_some(),
        self.pending_active_turn.is_some(),
      );
      final_stop_reason = Some(stop_reason);

      if matches!(step_result.control, TurnStepControl::Stop) || !stop_reason.can_continue() {
        break;
      }

      next_action =
        self.next_action_after_step(step_count, observation, step_result.next_action);
    }

    TurnLoopRunSummary {
      step_count,
      stop_reason: final_stop_reason.unwrap_or(AgentLoopStopReason::Completed),
    }
  }

  fn dispatch_action(&mut self, action: PreparedTurnAction) -> TurnStepResult {
    let mut dispatcher = TurnStepDispatcher::new(
      self.snapshot,
      self.items,
      self.pending_active_turn,
      self.pending_approval,
      self.plugin_command_output,
    );
    dispatcher.execute(action)
  }

  fn next_action_after_step(
    &self,
    _step_count: usize,
    observation: AgentLoopObservation,
    planned_next_action: Option<PreparedTurnAction>,
  ) -> Option<PreparedTurnAction> {
    if !observation.can_inform_next_action() {
      return None;
    }

    planned_next_action
  }
}
