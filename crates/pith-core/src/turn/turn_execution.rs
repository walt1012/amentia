use std::collections::HashMap;

use pith_protocol::TimelineItem;

use super::turn_approval_execution::{execute_shell_turn, execute_write_turn};
use super::turn_workspace_execution::{
  execute_list_turn, execute_no_workspace_turn, execute_read_turn, execute_search_turn,
  execute_web_search_turn,
};
use crate::request_state::{PreparedTurnAction, PreparedTurnSnapshot, TurnStartExecutionOutput};

pub(crate) fn execute_prepared_turn_snapshot(
  snapshot: PreparedTurnSnapshot,
) -> TurnStartExecutionOutput {
  let mut items = vec![TimelineItem {
    kind: "userMessage".to_string(),
    title: "User".to_string(),
    content: snapshot.display_message.clone(),
    attributes: None,
  }];
  let mut pending_active_turn = None;
  let mut pending_approval = None;

  match (&snapshot.workspace, &snapshot.action) {
    (
      Some(workspace),
      PreparedTurnAction::Write {
        intent,
        approval_id,
      },
    ) => {
      execute_write_turn(
        &snapshot,
        workspace,
        intent,
        approval_id,
        &mut items,
        &mut pending_approval,
      );
    }
    (
      Some(workspace),
      PreparedTurnAction::Shell {
        command,
        approval_id,
      },
    ) => {
      execute_shell_turn(
        &snapshot,
        workspace,
        command,
        approval_id,
        &mut items,
        &mut pending_approval,
      );
    }
    (Some(workspace), PreparedTurnAction::ReadFile { relative_path }) => {
      execute_read_turn(
        &snapshot,
        workspace,
        relative_path,
        &mut items,
        &mut pending_active_turn,
      );
    }
    (Some(workspace), PreparedTurnAction::Search { query }) => {
      execute_search_turn(
        &snapshot,
        workspace,
        query,
        &mut items,
        &mut pending_active_turn,
      );
    }
    (_, PreparedTurnAction::WebSearch(intent)) => {
      execute_web_search_turn(&snapshot, intent, &mut items, &mut pending_active_turn);
    }
    (Some(workspace), PreparedTurnAction::ListWorkspace) => {
      execute_list_turn(&snapshot, workspace, &mut items, &mut pending_active_turn);
    }
    _ => execute_no_workspace_turn(&snapshot, &mut items),
  }

  TurnStartExecutionOutput {
    thread_id: snapshot.thread_id,
    turn_id: snapshot.turn_id,
    items,
    pending_approval,
    pending_active_turn,
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
  }
}
