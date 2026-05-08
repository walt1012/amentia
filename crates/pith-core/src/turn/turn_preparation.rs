use std::collections::HashMap;
use std::path::Path;

use pith_protocol::WorkspaceSummary;

use crate::intent_inference::{
  infer_explicit_web_search_intent, infer_fresh_web_search_intent, infer_requested_file_path,
  infer_search_query, infer_shell_command, infer_write_intent,
};
use crate::plugin_permissions::permission_is_granted;
use crate::request_state::PreparedTurnAction;
use crate::runtime_context::RuntimeContext;

pub(crate) fn prepare_turn_action(
  context: &mut RuntimeContext,
  message: &str,
  workspace: Option<&WorkspaceSummary>,
  permission_sources: &HashMap<String, Vec<String>>,
) -> PreparedTurnAction {
  if let Some(intent) = infer_explicit_web_search_intent(message) {
    return PreparedTurnAction::WebSearch(intent);
  }

  if let Some(intent) = infer_fresh_web_search_intent(message) {
    return PreparedTurnAction::WebSearch(intent);
  }

  let Some(workspace) = workspace else {
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

  if let Some(query) = infer_search_query(message) {
    return PreparedTurnAction::Search { query };
  }

  PreparedTurnAction::ListWorkspace
}

fn reserve_approval_id(context: &mut RuntimeContext) -> String {
  context.sequence_state.next_approval_id()
}
