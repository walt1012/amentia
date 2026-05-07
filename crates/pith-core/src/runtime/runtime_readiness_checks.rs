use pith_protocol::RuntimeReadinessCheck;
use pith_sandbox::NativeSandboxStatus;

use crate::runtime_context::RuntimeContext;

pub(super) fn local_model_check(
  model_ready: bool,
  display_name: &str,
  backend: &str,
) -> RuntimeReadinessCheck {
  let status = if model_ready {
    "ready"
  } else {
    "setup_required"
  };

  RuntimeReadinessCheck {
    id: "localModel".to_string(),
    title: "Local Model".to_string(),
    status: status.to_string(),
    detail: if model_ready {
      format!("{display_name} is ready through {backend}.")
    } else {
      "Download and select one local model before agent work starts.".to_string()
    },
  }
}

pub(super) fn workspace_check(context: &RuntimeContext) -> RuntimeReadinessCheck {
  let status = if context.workspace_state.is_open() {
    "ready"
  } else {
    "setup_required"
  };

  RuntimeReadinessCheck {
    id: "workspace".to_string(),
    title: "Workspace".to_string(),
    status: status.to_string(),
    detail: context
      .workspace_state
      .current()
      .map(|workspace| format!("Tools are bound to {}.", workspace.display_name))
      .unwrap_or_else(|| {
        "Open a workspace to bind file, shell, memory, and approvals.".to_string()
      }),
  }
}

pub(super) fn thread_check(
  thread_ready: bool,
  workspace_thread_count: usize,
) -> RuntimeReadinessCheck {
  RuntimeReadinessCheck {
    id: "thread".to_string(),
    title: "Thread".to_string(),
    status: if thread_ready {
      "ready".to_string()
    } else {
      "setup_required".to_string()
    },
    detail: if thread_ready {
      format!("{workspace_thread_count} thread(s) are bound to the current workspace.")
    } else {
      "Create or resume a thread bound to the current workspace.".to_string()
    },
  }
}

pub(super) fn first_request_check(
  first_request_sent: bool,
  thread_ready: bool,
) -> RuntimeReadinessCheck {
  RuntimeReadinessCheck {
    id: "firstRequest".to_string(),
    title: "First Request".to_string(),
    status: if first_request_sent {
      "ready".to_string()
    } else if thread_ready {
      "ready_to_send".to_string()
    } else {
      "waiting".to_string()
    },
    detail: if first_request_sent {
      "At least one local request has been sent in the current workspace.".to_string()
    } else if thread_ready {
      "Send one short local request to complete first-use setup.".to_string()
    } else {
      "Create or resume a workspace-bound thread before sending the first request.".to_string()
    },
  }
}

pub(super) fn context_check(
  context_window: &str,
  output_cap: &str,
) -> RuntimeReadinessCheck {
  RuntimeReadinessCheck {
    id: "context".to_string(),
    title: "Context".to_string(),
    status: "ready".to_string(),
    detail: format!(
      "Context packing uses a {context_window} token runtime window with {output_cap} output cap."
    ),
  }
}

pub(super) fn execution_control_check(
  pending_approval_count: usize,
  active_turn_count: usize,
) -> RuntimeReadinessCheck {
  RuntimeReadinessCheck {
    id: "executionControls".to_string(),
    title: "Execution Controls".to_string(),
    status: execution_control_status(pending_approval_count, active_turn_count).to_string(),
    detail: execution_control_detail(pending_approval_count, active_turn_count),
  }
}

pub(super) fn plugin_check(
  enabled_plugin_count: usize,
  plugin_count: usize,
) -> RuntimeReadinessCheck {
  let status = if enabled_plugin_count > 0 {
    "ready"
  } else {
    "optional"
  };

  RuntimeReadinessCheck {
    id: "plugins".to_string(),
    title: "Plugins".to_string(),
    status: status.to_string(),
    detail: format!("{enabled_plugin_count} enabled of {plugin_count} discovered plugin(s)."),
  }
}

pub(super) fn native_sandbox_check(status: &NativeSandboxStatus) -> RuntimeReadinessCheck {
  let check_status = if status.active {
    "ready"
  } else if status.available {
    "setup_required"
  } else {
    "limited"
  };

  RuntimeReadinessCheck {
    id: "nativeSandbox".to_string(),
    title: "Native Sandbox".to_string(),
    status: check_status.to_string(),
    detail: status.detail.clone(),
  }
}

pub(super) fn bounded_runtime_check() -> RuntimeReadinessCheck {
  RuntimeReadinessCheck {
    id: "boundedRuntime".to_string(),
    title: "Bounded Runtime".to_string(),
    status: "ready".to_string(),
    detail: "Shell and llama.cpp subprocesses use timeouts and cleanup.".to_string(),
  }
}

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

fn execution_control_status(
  pending_approval_count: usize,
  active_turn_count: usize,
) -> &'static str {
  if pending_approval_count > 0 {
    "needs_approval"
  } else if active_turn_count > 0 {
    "running"
  } else {
    "ready"
  }
}

fn execution_control_detail(
  pending_approval_count: usize,
  active_turn_count: usize,
) -> String {
  if pending_approval_count > 0 {
    return format!("{pending_approval_count} approval request(s) are pending.");
  }
  if active_turn_count > 0 {
    return format!("{active_turn_count} turn(s) are active and cancellable.");
  }

  "Risky actions require approval, and turns can be cancelled.".to_string()
}
