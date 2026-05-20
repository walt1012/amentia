pub(super) struct ReadinessSummaryInput<'a> {
  pub(super) status: &'a str,
  pub(super) model_ready: bool,
  pub(super) workspace_ready: bool,
  pub(super) thread_ready: bool,
  pub(super) first_request_sent: bool,
  pub(super) pending_approval_count: usize,
  pub(super) active_turn_count: usize,
  pub(super) running_turn_count: usize,
  pub(super) running_approval_count: usize,
  pub(super) running_plugin_command_count: usize,
  pub(super) running_workspace_search_count: usize,
}

pub(super) fn readiness_summary(input: ReadinessSummaryInput<'_>) -> String {
  let ReadinessSummaryInput {
    status,
    model_ready,
    workspace_ready,
    thread_ready,
    first_request_sent,
    pending_approval_count,
    active_turn_count,
    running_turn_count,
    running_approval_count,
    running_plugin_command_count,
    running_workspace_search_count,
  } = input;

  match status {
    "setup_required" if !model_ready => {
      "Download and select one local model to enable local agent work.".to_string()
    }
    "setup_required" if !workspace_ready => {
      "Open a workspace so tools, memory, and approvals are scoped safely.".to_string()
    }
    "setup_required" if !thread_ready => {
      "Create or resume a workspace-bound thread before local agent work.".to_string()
    }
    "needs_approval" => {
      format!("Runtime is waiting on {pending_approval_count} approval(s) before continuing.")
    }
    "running" => {
      let active_execution_count = active_turn_count
        .saturating_add(running_turn_count)
        .saturating_add(running_approval_count)
        .saturating_add(running_plugin_command_count)
        .saturating_add(running_workspace_search_count);
      format!("Runtime is running {active_execution_count} local execution(s).")
    }
    "ready" if !first_request_sent => "Runtime ready for the first local request.".to_string(),
    _ => "Runtime ready: model, workspace, tools, context, and plugins are controlled.".to_string(),
  }
}
