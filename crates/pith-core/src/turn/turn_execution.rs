use std::collections::HashMap;

use pith_protocol::TimelineItem;

use super::turn_agent_loop::AgentLoopCoordinator;
use super::turn_approval_execution::{execute_shell_turn, execute_write_turn};
use super::turn_web_search::{
  execute_web_search_candidate_local_answer, model_confirms_web_search_candidate,
};
use super::turn_workspace_execution::{
  execute_list_turn, execute_no_workspace_turn, execute_read_turn, execute_search_turn,
  execute_web_search_turn,
};
use crate::plugin_commands::execute_plugin_command_snapshot;
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
  let step_start_index = items.len();
  let agent_loop = AgentLoopCoordinator::new(&snapshot.turn_id);
  let agent_step = agent_loop.begin_compatibility_step(&action);

  match action {
    PreparedTurnAction::Write {
      intent,
      approval_id,
    } => {
      if let Some(workspace) = snapshot.workspace.as_ref() {
        execute_write_turn(
          &snapshot,
          workspace,
          &intent,
          &approval_id,
          &mut items,
          &mut pending_approval,
        );
      } else {
        execute_no_workspace_turn(&snapshot, &mut items);
      }
    }
    PreparedTurnAction::Shell {
      command,
      approval_id,
    } => {
      if let Some(workspace) = snapshot.workspace.as_ref() {
        execute_shell_turn(
          &snapshot,
          workspace,
          &command,
          &approval_id,
          &mut items,
          &mut pending_approval,
        );
      } else {
        execute_no_workspace_turn(&snapshot, &mut items);
      }
    }
    PreparedTurnAction::PluginCommand { snapshot } => {
      let command_id = snapshot.command_id().to_string();
      match execute_plugin_command_snapshot(*snapshot) {
        Ok(output) => {
          pending_approval = output.pending_approval.clone();
          let should_capture_memory = output.capture_memory;
          items.extend(output.items.clone());
          if should_capture_memory {
            plugin_command_output = Some(output);
          }
        }
        Err((code, message)) => {
          items.push(TimelineItem {
            kind: "warning".to_string(),
            title: "Plugin Command Failed".to_string(),
            content: message.clone(),
            attributes: Some(HashMap::from([
              ("commandId".to_string(), command_id),
              ("errorCode".to_string(), code.to_string()),
            ])),
          });
          items.push(TimelineItem {
            kind: "assistantMessage".to_string(),
            title: "Assistant".to_string(),
            content: "The plugin command failed before it could produce output. Inspect the command setup and retry."
              .to_string(),
            attributes: None,
          });
        }
      }
    }
    PreparedTurnAction::PluginCommandRouteFailed {
      command_id,
      message,
      attributes,
    } => {
      items.push(TimelineItem {
        kind: "warning".to_string(),
        title: "Plugin Command Not Ready".to_string(),
        content: message,
        attributes: Some(attributes),
      });
      items.push(TimelineItem {
        kind: "assistantMessage".to_string(),
        title: "Assistant".to_string(),
        content: format!(
          "Pith could not run `{command_id}` yet. Fix the plugin setup shown above, then retry the same command."
        ),
        attributes: None,
      });
    }
    PreparedTurnAction::ReadFile { relative_path } => {
      if let Some(workspace) = snapshot.workspace.as_ref() {
        execute_read_turn(
          &snapshot,
          workspace,
          &relative_path,
          &mut items,
          &mut pending_active_turn,
        );
      } else {
        execute_no_workspace_turn(&snapshot, &mut items);
      }
    }
    PreparedTurnAction::Search { query } => {
      if let Some(workspace) = snapshot.workspace.as_ref() {
        execute_search_turn(
          &snapshot,
          workspace,
          &query,
          &mut items,
          &mut pending_active_turn,
        );
      } else {
        execute_no_workspace_turn(&snapshot, &mut items);
      }
    }
    PreparedTurnAction::WebSearch(intent) => {
      execute_web_search_turn(&snapshot, &intent, &mut items, &mut pending_active_turn);
    }
    PreparedTurnAction::WebSearchCandidate(intent) => {
      if model_confirms_web_search_candidate(&snapshot, &intent) {
        execute_web_search_turn(&snapshot, &intent, &mut items, &mut pending_active_turn);
      } else {
        execute_web_search_candidate_local_answer(
          &snapshot,
          &intent,
          &mut items,
          &mut pending_active_turn,
        );
      }
    }
    PreparedTurnAction::ListWorkspace => {
      if let Some(workspace) = snapshot.workspace.as_ref() {
        execute_list_turn(&snapshot, workspace, &mut items, &mut pending_active_turn);
      } else {
        execute_no_workspace_turn(&snapshot, &mut items);
      }
    }
    PreparedTurnAction::NoWorkspace => execute_no_workspace_turn(&snapshot, &mut items),
  }

  agent_loop.finish_compatibility_step(
    &agent_step,
    &mut items[step_start_index..],
    pending_approval.is_some(),
    pending_active_turn.is_some(),
  );

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
