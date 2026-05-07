use pith_protocol::TimelineItem;

use super::turn_approval_execution::{execute_shell_turn, execute_write_turn};
use super::turn_workspace_execution::{
  execute_list_turn, execute_no_workspace_turn, execute_read_turn, execute_search_turn,
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
