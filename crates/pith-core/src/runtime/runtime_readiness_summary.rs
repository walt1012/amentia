pub(super) fn readiness_summary(
  status: &str,
  model_ready: bool,
  workspace_ready: bool,
  thread_ready: bool,
  first_request_sent: bool,
  pending_approval_count: usize,
  active_turn_count: usize,
) -> String {
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
    "running" => format!("Runtime is running {active_turn_count} active turn(s)."),
    "ready" if !first_request_sent => "Runtime ready for the first local request.".to_string(),
    _ => "Runtime ready: model, workspace, tools, context, and plugins are controlled.".to_string(),
  }
}
