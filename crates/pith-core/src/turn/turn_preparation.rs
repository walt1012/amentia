use std::collections::HashMap;

use pith_model_runtime::GenerationCancellation;
use pith_protocol::WorkspaceSummary;

use super::turn_plugin_routing::ExplicitPluginCommandRoute;
use super::turn_tool_planning::{plan_initial_turn_tool, InitialToolPlan};
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
  let plan = plan_initial_turn_tool(message, workspace, context.plugin_state.catalog());
  materialize_initial_tool_plan(
    context,
    thread_id,
    workspace,
    permission_sources,
    cancellation,
    plan,
  )
}

fn materialize_initial_tool_plan(
  context: &mut RuntimeContext,
  thread_id: &str,
  workspace: Option<&WorkspaceSummary>,
  permission_sources: &HashMap<String, Vec<String>>,
  cancellation: GenerationCancellation,
  plan: InitialToolPlan,
) -> PreparedTurnAction {
  match plan {
    InitialToolPlan::WebSearch { intent } => PreparedTurnAction::WebSearch(intent),
    InitialToolPlan::PluginCommand { route } => {
      prepare_plugin_route_action(context, thread_id, workspace, route, cancellation)
    }
    InitialToolPlan::NoWorkspace => PreparedTurnAction::NoWorkspace,
    InitialToolPlan::Write { intent } => {
      let approval_id = permission_is_granted(permission_sources, "file.write")
        .then(|| reserve_approval_id(context));
      PreparedTurnAction::Write {
        intent,
        approval_id,
      }
    }
    InitialToolPlan::Shell { command } => {
      let approval_id = permission_is_granted(permission_sources, "shell.exec")
        .then(|| reserve_approval_id(context));
      PreparedTurnAction::Shell {
        command,
        approval_id,
      }
    }
    InitialToolPlan::ReadFile { relative_path } => PreparedTurnAction::ReadFile { relative_path },
    InitialToolPlan::Search { query } => PreparedTurnAction::Search { query },
    InitialToolPlan::WebSearchCandidate { intent } => {
      PreparedTurnAction::WebSearchCandidate(intent)
    }
    InitialToolPlan::ListWorkspace => PreparedTurnAction::ListWorkspace,
  }
}

fn reserve_approval_id(context: &mut RuntimeContext) -> String {
  context.sequence_state.next_approval_id()
}

fn prepare_plugin_route_action(
  context: &mut RuntimeContext,
  thread_id: &str,
  workspace: Option<&WorkspaceSummary>,
  route: ExplicitPluginCommandRoute,
  cancellation: GenerationCancellation,
) -> PreparedTurnAction {
  let command_id = route.command_id;
  let input = route.input;
  let route_input = input.clone();
  let routing_reason = route.routing_reason;
  match prepare_plugin_command_turn_snapshot(
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
  }
}
