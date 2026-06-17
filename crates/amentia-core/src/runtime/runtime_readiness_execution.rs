use amentia_protocol::RuntimeReadinessCheck;

const DEFAULT_EXECUTION_CONTROL_DETAIL: &str =
  "Default mode: Amentia asks before changing files; risky actions need approval and can be cancelled.";

pub(super) fn execution_control_check(
  pending_approval_count: usize,
  active_turn_count: usize,
  running_turn_count: usize,
  running_approval_count: usize,
  running_plugin_command_count: usize,
  running_workspace_search_count: usize,
) -> RuntimeReadinessCheck {
  RuntimeReadinessCheck {
    id: "executionControls".to_string(),
    title: "Action Safety".to_string(),
    status: execution_control_status(
      pending_approval_count,
      active_turn_count,
      running_turn_count,
      running_approval_count,
      running_plugin_command_count,
      running_workspace_search_count,
    )
    .to_string(),
    detail: execution_control_detail(
      pending_approval_count,
      active_turn_count,
      running_turn_count,
      running_approval_count,
      running_plugin_command_count,
      running_workspace_search_count,
    ),
  }
}

pub(super) fn bounded_runtime_check() -> RuntimeReadinessCheck {
  RuntimeReadinessCheck {
    id: "boundedRuntime".to_string(),
    title: "Reliable Local Work".to_string(),
    status: "ready".to_string(),
    detail: "Shell, llama.cpp, web search, git helpers, and plugin actions are bounded."
      .to_string(),
  }
}

fn execution_control_status(
  pending_approval_count: usize,
  active_turn_count: usize,
  running_turn_count: usize,
  running_approval_count: usize,
  running_plugin_command_count: usize,
  running_workspace_search_count: usize,
) -> &'static str {
  if pending_approval_count > 0 {
    "needs_approval"
  } else if active_turn_count > 0
    || running_turn_count > 0
    || running_approval_count > 0
    || running_plugin_command_count > 0
    || running_workspace_search_count > 0
  {
    "running"
  } else {
    "ready"
  }
}

fn execution_control_detail(
  pending_approval_count: usize,
  active_turn_count: usize,
  running_turn_count: usize,
  running_approval_count: usize,
  running_plugin_command_count: usize,
  running_workspace_search_count: usize,
) -> String {
  if pending_approval_count > 0 {
    return format!("{pending_approval_count} approval request(s) are pending.");
  }
  if active_turn_count > 0 {
    return format!("{active_turn_count} request response(s) are streaming and cancellable.");
  }
  if running_turn_count > 0 {
    return format!("{running_turn_count} request execution(s) are active and cancellable.");
  }
  if running_approval_count > 0 {
    return format!("{running_approval_count} approval execution(s) are active and cancellable.");
  }
  if running_plugin_command_count > 0 {
    return format!(
      "{running_plugin_command_count} plugin action execution(s) are active and cancellable."
    );
  }
  if running_workspace_search_count > 0 {
    return format!(
      "{running_workspace_search_count} project search(es) are active and cancellable."
    );
  }

  DEFAULT_EXECUTION_CONTROL_DETAIL.to_string()
}
