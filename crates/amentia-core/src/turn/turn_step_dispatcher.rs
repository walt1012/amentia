use std::collections::HashMap;

use amentia_protocol::TimelineItem;

use super::turn_approval_shell::execute_shell_turn;
use super::turn_approval_write::execute_write_turn;
use super::turn_web_search::{
  execute_web_search_candidate_local_answer, execute_web_search_turn,
  model_confirms_web_search_candidate,
};
use super::turn_workspace_fallback::execute_no_workspace_turn;
use super::turn_workspace_list::execute_list_observation_step;
use super::turn_workspace_read::execute_read_observation_step;
use super::turn_workspace_search::execute_search_observation_step;
use crate::active_turns::ActiveTurn;
use crate::approval_types::PendingApproval;
use crate::plugin_commands::{
  ensure_plugin_command_handoff, execute_plugin_command_snapshot, PluginCommandOutput,
  PluginCommandSnapshot,
};
use crate::request_state::{PreparedTurnAction, PreparedTurnSnapshot};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TurnStepControl {
  Continue,
  Stop,
}

#[derive(Debug)]
pub(crate) struct TurnStepResult {
  pub(crate) control: TurnStepControl,
  pub(crate) next_action: Option<PreparedTurnAction>,
}

impl TurnStepResult {
  fn new(control: TurnStepControl) -> Self {
    Self {
      control,
      next_action: None,
    }
  }

  fn with_next_action(control: TurnStepControl, next_action: PreparedTurnAction) -> Self {
    Self {
      control,
      next_action: Some(next_action),
    }
  }
}

pub(super) struct TurnStepDispatcher<'a> {
  snapshot: &'a PreparedTurnSnapshot,
  items: &'a mut Vec<TimelineItem>,
  pending_active_turn: &'a mut Option<ActiveTurn>,
  pending_approval: &'a mut Option<PendingApproval>,
  plugin_command_outputs: &'a mut Vec<PluginCommandOutput>,
}

impl<'a> TurnStepDispatcher<'a> {
  pub(super) fn new(
    snapshot: &'a PreparedTurnSnapshot,
    items: &'a mut Vec<TimelineItem>,
    pending_active_turn: &'a mut Option<ActiveTurn>,
    pending_approval: &'a mut Option<PendingApproval>,
    plugin_command_outputs: &'a mut Vec<PluginCommandOutput>,
  ) -> Self {
    Self {
      snapshot,
      items,
      pending_active_turn,
      pending_approval,
      plugin_command_outputs,
    }
  }

  pub(super) fn execute(&mut self, action: PreparedTurnAction) -> TurnStepResult {
    match action {
      PreparedTurnAction::Write { intent, policy } => {
        if let Some(workspace) = self.snapshot.workspace.as_ref() {
          execute_write_turn(
            self.snapshot,
            workspace,
            &intent,
            &policy,
            self.items,
            self.pending_approval,
          );
        } else {
          execute_no_workspace_turn(self.snapshot, self.items);
        }
      }
      PreparedTurnAction::Shell { command, policy } => {
        if let Some(workspace) = self.snapshot.workspace.as_ref() {
          execute_shell_turn(
            self.snapshot,
            workspace,
            &command,
            &policy,
            self.items,
            self.pending_approval,
          );
        } else {
          execute_no_workspace_turn(self.snapshot, self.items);
        }
      }
      PreparedTurnAction::PluginCommand { snapshot } => {
        self.execute_plugin_command(*snapshot);
      }
      PreparedTurnAction::PluginCommandRouteFailed {
        command_id,
        message,
        attributes,
      } => {
        self.execute_plugin_command_route_failed(command_id, message, attributes);
      }
      PreparedTurnAction::ReadFile { relative_path } => {
        let next_action = if let Some(workspace) = self.snapshot.workspace.as_ref() {
          execute_read_observation_step(
            self.snapshot,
            workspace,
            &relative_path,
            self.items,
            self.pending_active_turn,
          )
          .map(|relative_path| PreparedTurnAction::ReadFile { relative_path })
        } else {
          execute_no_workspace_turn(self.snapshot, self.items);
          None
        };
        let control = self.control_after_step();
        return if let Some(next_action) = next_action {
          TurnStepResult::with_next_action(control, next_action)
        } else {
          TurnStepResult::new(control)
        };
      }
      PreparedTurnAction::Search { query } => {
        let next_action = if let Some(workspace) = self.snapshot.workspace.as_ref() {
          execute_search_observation_step(
            self.snapshot,
            workspace,
            &query,
            self.items,
            self.pending_active_turn,
          )
          .map(|relative_path| PreparedTurnAction::ReadFile { relative_path })
        } else {
          execute_no_workspace_turn(self.snapshot, self.items);
          None
        };
        let control = self.control_after_step();
        return if let Some(next_action) = next_action {
          TurnStepResult::with_next_action(control, next_action)
        } else {
          TurnStepResult::new(control)
        };
      }
      PreparedTurnAction::WebSearch(intent) => {
        execute_web_search_turn(self.snapshot, &intent, self.items, self.pending_active_turn);
      }
      PreparedTurnAction::WebSearchCandidate(intent) => {
        if model_confirms_web_search_candidate(self.snapshot, &intent) {
          execute_web_search_turn(self.snapshot, &intent, self.items, self.pending_active_turn);
        } else {
          execute_web_search_candidate_local_answer(
            self.snapshot,
            &intent,
            self.items,
            self.pending_active_turn,
          );
        }
      }
      PreparedTurnAction::ListWorkspace => {
        let next_action = if let Some(workspace) = self.snapshot.workspace.as_ref() {
          execute_list_observation_step(
            self.snapshot,
            workspace,
            self.items,
            self.pending_active_turn,
          )
          .map(|relative_path| PreparedTurnAction::ReadFile { relative_path })
        } else {
          execute_no_workspace_turn(self.snapshot, self.items);
          None
        };
        let control = self.control_after_step();
        return if let Some(next_action) = next_action {
          TurnStepResult::with_next_action(control, next_action)
        } else {
          TurnStepResult::new(control)
        };
      }
      PreparedTurnAction::NoWorkspace => execute_no_workspace_turn(self.snapshot, self.items),
    }

    TurnStepResult::new(self.control_after_step())
  }

  fn execute_plugin_command(&mut self, snapshot: PluginCommandSnapshot) {
    let command_id = snapshot.command_id().to_string();
    match execute_plugin_command_snapshot(snapshot) {
      Ok(output) => {
        let mut output = output;
        if output.pending_approval.is_none() {
          ensure_plugin_command_handoff(&mut output, "pluginCommand");
        }
        *self.pending_approval = output.pending_approval.clone();
        let should_capture_memory = output.capture_memory;
        self.items.extend(output.items.clone());
        if should_capture_memory {
          self.plugin_command_outputs.push(output);
        }
      }
      Err((code, message)) => {
        self.items.push(TimelineItem {
          kind: "warning".to_string(),
          title: "Plugin Action Failed".to_string(),
          content: message.clone(),
          attributes: Some(HashMap::from([
            ("commandId".to_string(), command_id),
            ("errorCode".to_string(), code.to_string()),
          ])),
        });
        self.items.push(TimelineItem {
          kind: "assistantMessage".to_string(),
          title: "Assistant".to_string(),
          content: "The plugin action failed before it could produce output. Inspect the plugin setup and retry."
            .to_string(),
          attributes: None,
        });
      }
    }
  }

  fn execute_plugin_command_route_failed(
    &mut self,
    command_id: String,
    message: String,
    attributes: HashMap<String, String>,
  ) {
    self.items.push(TimelineItem {
      kind: "warning".to_string(),
      title: "Plugin Action Not Ready".to_string(),
      content: message,
      attributes: Some(attributes),
    });
    self.items.push(TimelineItem {
      kind: "assistantMessage".to_string(),
      title: "Assistant".to_string(),
      content: format!(
        "Amentia could not run `{command_id}` yet. Fix the plugin setup shown above, then retry the same action."
      ),
      attributes: None,
    });
  }

  fn control_after_step(&self) -> TurnStepControl {
    if self.snapshot.cancellation.is_cancelled()
      || self.pending_active_turn.is_some()
      || self.pending_approval.is_some()
    {
      TurnStepControl::Stop
    } else {
      TurnStepControl::Continue
    }
  }
}
