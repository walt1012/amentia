use std::collections::HashMap;
use std::path::Path;

use pith_model_runtime::llama_cpp_timeout_seconds;
use pith_protocol::{RuntimeReadinessCheck, RuntimeReadinessResult};
use pith_sandbox::workspace_required_status;
use pith_tools::{shell_command_timeout_seconds, shell_sandbox_status};

use crate::runtime_context::RuntimeContext;
use crate::thread_state::StoredThread;

pub(crate) fn build_runtime_readiness(context: &RuntimeContext) -> RuntimeReadinessResult {
  let model_health = context.model_runtime.health();
  let model_ready = model_health.status == "ready";
  let workspace_ready = context.workspace.is_some();
  let workspace_thread_count = count_workspace_threads(context);
  let thread_ready = workspace_thread_count > 0;
  let first_request_sent = has_first_request(context);
  let pending_approval_count = context.pending_approvals.len();
  let active_turn_count = context.active_turns.len();
  let sandbox_status = context
    .workspace
    .as_ref()
    .map(|workspace| shell_sandbox_status(Path::new(&workspace.root_path)))
    .unwrap_or_else(workspace_required_status);
  let enabled_plugin_count = context
    .plugins
    .iter()
    .filter(|plugin| plugin.enabled && plugin.status == "ready")
    .count();

  let status = if !model_ready || !workspace_ready || !thread_ready {
    "setup_required"
  } else if pending_approval_count > 0 {
    "needs_approval"
  } else if active_turn_count > 0 {
    "running"
  } else {
    "ready"
  };
  let context_window = model_health
    .metrics
    .get("contextSize")
    .cloned()
    .unwrap_or_else(|| "4096".to_string());
  let output_cap = model_health
    .metrics
    .get("maxOutputTokens")
    .cloned()
    .unwrap_or_else(|| "unknown".to_string());

  RuntimeReadinessResult {
    status: status.to_string(),
    summary: readiness_summary(
      status,
      model_ready,
      workspace_ready,
      thread_ready,
      first_request_sent,
      pending_approval_count,
      active_turn_count,
    ),
    checks: vec![
      local_model_check(
        model_ready,
        &model_health.display_name,
        &model_health.backend,
      ),
      workspace_check(context),
      thread_check(thread_ready, workspace_thread_count),
      first_request_check(first_request_sent, thread_ready),
      context_check(&context_window, &output_cap),
      execution_control_check(pending_approval_count, active_turn_count),
      native_sandbox_check(&sandbox_status),
      plugin_check(enabled_plugin_count, context.plugins.len()),
      bounded_runtime_check(),
    ],
    metrics: readiness_metrics(
      context,
      &model_health.status,
      &model_health.pack_id,
      &context_window,
      enabled_plugin_count,
      &sandbox_status,
      workspace_thread_count,
      first_request_sent,
    ),
  }
}

fn local_model_check(
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

fn workspace_check(context: &RuntimeContext) -> RuntimeReadinessCheck {
  let status = if context.workspace.is_some() {
    "ready"
  } else {
    "setup_required"
  };

  RuntimeReadinessCheck {
    id: "workspace".to_string(),
    title: "Workspace".to_string(),
    status: status.to_string(),
    detail: context
      .workspace
      .as_ref()
      .map(|workspace| format!("Tools are bound to {}.", workspace.display_name))
      .unwrap_or_else(|| {
        "Open a workspace to bind file, shell, memory, and approvals.".to_string()
      }),
  }
}

fn thread_check(thread_ready: bool, workspace_thread_count: usize) -> RuntimeReadinessCheck {
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

fn first_request_check(first_request_sent: bool, thread_ready: bool) -> RuntimeReadinessCheck {
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

fn context_check(context_window: &str, output_cap: &str) -> RuntimeReadinessCheck {
  RuntimeReadinessCheck {
    id: "context".to_string(),
    title: "Context".to_string(),
    status: "ready".to_string(),
    detail: format!(
      "Context packing uses a {context_window} token runtime window with {output_cap} output cap."
    ),
  }
}

fn execution_control_check(
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

fn plugin_check(enabled_plugin_count: usize, plugin_count: usize) -> RuntimeReadinessCheck {
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

fn native_sandbox_check(status: &pith_sandbox::NativeSandboxStatus) -> RuntimeReadinessCheck {
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

fn bounded_runtime_check() -> RuntimeReadinessCheck {
  RuntimeReadinessCheck {
    id: "boundedRuntime".to_string(),
    title: "Bounded Runtime".to_string(),
    status: "ready".to_string(),
    detail: "Shell and llama.cpp subprocesses use timeouts and cleanup.".to_string(),
  }
}

fn readiness_summary(
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

fn execution_control_detail(pending_approval_count: usize, active_turn_count: usize) -> String {
  if pending_approval_count > 0 {
    return format!("{pending_approval_count} approval request(s) are pending.");
  }
  if active_turn_count > 0 {
    return format!("{active_turn_count} turn(s) are active and cancellable.");
  }

  "Risky actions require approval, and turns can be cancelled.".to_string()
}

fn readiness_metrics(
  context: &RuntimeContext,
  model_status: &str,
  model_pack_id: &str,
  context_window: &str,
  enabled_plugin_count: usize,
  sandbox_status: &pith_sandbox::NativeSandboxStatus,
  workspace_thread_count: usize,
  first_request_sent: bool,
) -> HashMap<String, String> {
  let mut metrics = HashMap::from([
    ("modelStatus".to_string(), model_status.to_string()),
    ("modelPackId".to_string(), model_pack_id.to_string()),
    (
      "workspaceBound".to_string(),
      context.workspace.is_some().to_string(),
    ),
    (
      "pendingApprovalCount".to_string(),
      context.pending_approvals.len().to_string(),
    ),
    (
      "activeTurnCount".to_string(),
      context.active_turns.len().to_string(),
    ),
    (
      "workspaceThreadCount".to_string(),
      workspace_thread_count.to_string(),
    ),
    (
      "firstRequestSent".to_string(),
      first_request_sent.to_string(),
    ),
    (
      "memoryNoteCount".to_string(),
      context.memory_notes.len().to_string(),
    ),
    ("pluginCount".to_string(), context.plugins.len().to_string()),
    (
      "enabledPluginCount".to_string(),
      enabled_plugin_count.to_string(),
    ),
    ("sandboxMode".to_string(), sandbox_status.mode.clone()),
    ("sandboxBackend".to_string(), sandbox_status.backend.clone()),
    (
      "sandboxAvailable".to_string(),
      sandbox_status.available.to_string(),
    ),
    (
      "sandboxActive".to_string(),
      sandbox_status.active.to_string(),
    ),
    (
      "contextWindowTokens".to_string(),
      context_window.to_string(),
    ),
    (
      "shellTimeoutSeconds".to_string(),
      shell_command_timeout_seconds().to_string(),
    ),
    (
      "llamaTimeoutSeconds".to_string(),
      llama_cpp_timeout_seconds().to_string(),
    ),
  ]);
  if let Some(temporary_root) = &sandbox_status.temporary_root {
    metrics.insert("sandboxTempRoot".to_string(), temporary_root.clone());
  }
  metrics
}

fn count_workspace_threads(context: &RuntimeContext) -> usize {
  current_workspace_threads(context).count()
}

fn has_first_request(context: &RuntimeContext) -> bool {
  current_workspace_threads(context)
    .any(|thread| thread.items.iter().any(|item| item.kind == "userMessage"))
}

fn current_workspace_threads(context: &RuntimeContext) -> impl Iterator<Item = &StoredThread> + '_ {
  context.threads.iter().filter(move |thread| {
    let Some(workspace) = &context.workspace else {
      return false;
    };

    thread
      .workspace
      .as_ref()
      .or(thread.summary.workspace.as_ref())
      .map(|thread_workspace| thread_workspace.root_path == workspace.root_path)
      .unwrap_or(false)
  })
}
