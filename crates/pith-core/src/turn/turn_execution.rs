use std::collections::HashMap;

use pith_protocol::TimelineItem;

use super::turn_agent_loop::{AgentLoopStopReason, LOOP_MAX_STEPS};
use super::turn_loop_runner::TurnLoopRunner;
use crate::request_state::{PreparedTurnAction, PreparedTurnSnapshot, TurnStartExecutionOutput};

pub(crate) fn execute_prepared_turn_snapshot(
  mut snapshot: PreparedTurnSnapshot,
) -> TurnStartExecutionOutput {
  let mut items = vec![TimelineItem {
    kind: "userMessage".to_string(),
    title: "User".to_string(),
    content: snapshot.display_message.clone(),
    attributes: None,
  }];
  let mut pending_active_turn = None;
  let mut pending_approval = None;
  let mut plugin_command_output = None;
  let action = std::mem::replace(&mut snapshot.action, PreparedTurnAction::NoWorkspace);
  {
    let mut runner = TurnLoopRunner::new(
      &snapshot,
      &mut items,
      &mut pending_active_turn,
      &mut pending_approval,
      &mut plugin_command_output,
    );
    let loop_summary = runner.run(action);
    debug_assert!(
      loop_summary.stop_reason != AgentLoopStopReason::StepBudgetExhausted
        || loop_summary.step_count >= LOOP_MAX_STEPS
    );
  }

  TurnStartExecutionOutput {
    thread_id: snapshot.thread_id,
    turn_id: snapshot.turn_id,
    items,
    pending_approval,
    pending_active_turn,
    plugin_command_output,
  }
}

pub(crate) fn build_recovered_turn_output(
  thread_id: String,
  turn_id: String,
  display_message: String,
) -> TurnStartExecutionOutput {
  TurnStartExecutionOutput {
    thread_id,
    turn_id: turn_id.clone(),
    items: vec![
      TimelineItem {
        kind: "userMessage".to_string(),
        title: "User".to_string(),
        content: display_message,
        attributes: None,
      },
      TimelineItem {
        kind: "warning".to_string(),
        title: "Turn Recovered".to_string(),
        content: "Pith recovered this turn after an internal runtime error.".to_string(),
        attributes: Some(HashMap::from([
          ("turnId".to_string(), turn_id.clone()),
          ("recovery".to_string(), "runtimePanic".to_string()),
        ])),
      },
      TimelineItem {
        kind: "assistantMessage".to_string(),
        title: "Assistant".to_string(),
        content: "The local turn stopped before Pith could finish. Try again when ready."
          .to_string(),
        attributes: Some(HashMap::from([
          ("turnId".to_string(), turn_id),
          ("runtimeRecovered".to_string(), "true".to_string()),
        ])),
      },
    ],
    pending_approval: None,
    pending_active_turn: None,
    plugin_command_output: None,
  }
}
