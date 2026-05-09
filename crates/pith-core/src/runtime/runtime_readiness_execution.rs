use pith_protocol::RuntimeReadinessCheck;

pub(super) fn execution_control_check(
  pending_approval_count: usize,
  active_turn_count: usize,
  running_approval_count: usize,
) -> RuntimeReadinessCheck {
  RuntimeReadinessCheck {
    id: "executionControls".to_string(),
    title: "Execution Controls".to_string(),
    status: execution_control_status(
      pending_approval_count,
      active_turn_count,
      running_approval_count,
    )
    .to_string(),
    detail: execution_control_detail(
      pending_approval_count,
      active_turn_count,
      running_approval_count,
    ),
  }
}

pub(super) fn bounded_runtime_check() -> RuntimeReadinessCheck {
  RuntimeReadinessCheck {
    id: "boundedRuntime".to_string(),
    title: "Bounded Runtime".to_string(),
    status: "ready".to_string(),
    detail: "Shell, llama.cpp, web search, and git helpers are bounded.".to_string(),
  }
}

fn execution_control_status(
  pending_approval_count: usize,
  active_turn_count: usize,
  running_approval_count: usize,
) -> &'static str {
  if pending_approval_count > 0 {
    "needs_approval"
  } else if active_turn_count > 0 || running_approval_count > 0 {
    "running"
  } else {
    "ready"
  }
}

fn execution_control_detail(
  pending_approval_count: usize,
  active_turn_count: usize,
  running_approval_count: usize,
) -> String {
  if pending_approval_count > 0 {
    return format!("{pending_approval_count} approval request(s) are pending.");
  }
  if active_turn_count > 0 {
    return format!("{active_turn_count} turn(s) are active and cancellable.");
  }
  if running_approval_count > 0 {
    return format!("{running_approval_count} approval execution(s) are active and cancellable.");
  }

  "Risky actions require approval, and local executions can be cancelled.".to_string()
}
