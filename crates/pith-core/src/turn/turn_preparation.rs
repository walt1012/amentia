use std::collections::HashMap;
use std::path::Path;

use pith_model_runtime::GenerationCancellation;
use pith_protocol::WorkspaceSummary;

use super::turn_plugin_routing::infer_explicit_plugin_command_route;
use crate::intent_inference::{
  infer_explicit_web_search_intent, infer_fresh_web_search_intent, infer_model_web_search_intent,
  infer_requested_file_path, infer_search_query, infer_shell_command, infer_write_intent,
};
use crate::plugin_commands::prepare_plugin_command_turn_snapshot;
use crate::plugin_permissions::permission_is_granted;
use crate::request_state::PreparedTurnAction;
use crate::runtime_context::RuntimeContext;

pub(crate) fn prepare_turn_action(
  context: &mut RuntimeContext,
  thread_id: &str,
  message: &str,
  workspace: Option<&WorkspaceSummary>,
  permission_sources: &HashMap<String, Vec<String>>,
  cancellation: GenerationCancellation,
) -> PreparedTurnAction {
  if let Some(intent) = infer_explicit_web_search_intent(message) {
    return PreparedTurnAction::WebSearch(intent);
  }

  if let Some(route) = infer_explicit_plugin_command_route(message) {
    let command_id = route.command_id;
    let input = route.input;
    let route_input = input.clone();
    let routing_reason = route.routing_reason;
    return match prepare_plugin_command_turn_snapshot(
      context,
      thread_id,
      workspace.cloned(),
      &command_id,
      input,
      cancellation,
    ) {
      Ok(snapshot) => PreparedTurnAction::PluginCommand {
        snapshot: Box::new(snapshot),
      },
      Err(error) => PreparedTurnAction::PluginCommandRouteFailed {
        attributes: error.route_failure_attributes(
          &command_id,
          routing_reason,
          route_input.as_deref(),
        ),
        command_id,
        message: error.message().to_string(),
      },
    };
  }

  let Some(workspace) = workspace else {
    if let Some(intent) = infer_fresh_web_search_intent(message) {
      return PreparedTurnAction::WebSearchCandidate(intent);
    }
    if let Some(intent) = infer_model_web_search_intent(message) {
      return PreparedTurnAction::WebSearchCandidate(intent);
    }
    return PreparedTurnAction::NoWorkspace;
  };
  let workspace_root = Path::new(&workspace.root_path);

  if let Some(intent) = infer_write_intent(message) {
    let approval_id =
      permission_is_granted(permission_sources, "file.write").then(|| reserve_approval_id(context));
    return PreparedTurnAction::Write {
      intent,
      approval_id,
    };
  }

  if let Some(command) = infer_shell_command(message) {
    let approval_id =
      permission_is_granted(permission_sources, "shell.exec").then(|| reserve_approval_id(context));
    return PreparedTurnAction::Shell {
      command,
      approval_id,
    };
  }

  if let Some(relative_path) = infer_requested_file_path(message, workspace_root) {
    return PreparedTurnAction::ReadFile { relative_path };
  }

  if let Some(intent) = infer_fresh_web_search_intent(message) {
    return PreparedTurnAction::WebSearchCandidate(intent);
  }

  if let Some(query) = infer_search_query(message) {
    return PreparedTurnAction::Search { query };
  }

  if let Some(intent) = infer_model_web_search_intent(message) {
    return PreparedTurnAction::WebSearchCandidate(intent);
  }

  PreparedTurnAction::ListWorkspace
}

fn reserve_approval_id(context: &mut RuntimeContext) -> String {
  context.sequence_state.next_approval_id()
}
