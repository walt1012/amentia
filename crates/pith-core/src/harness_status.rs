use std::collections::HashMap;

use pith_model_runtime::llama_cpp_timeout_seconds;
use pith_protocol::{HarnessCheck, HarnessStatusResult};
use pith_tools::shell_command_timeout_seconds;

use crate::RuntimeContext;

pub(crate) fn build_harness_status(context: &RuntimeContext) -> HarnessStatusResult {
  let model_health = context.model_runtime.health();
  let model_ready = model_health.status == "ready";
  let workspace_ready = context.workspace.is_some();
  let pending_approval_count = context.pending_approvals.len();
  let active_turn_count = context.active_turns.len();
  let enabled_plugin_count = context
    .plugins
    .iter()
    .filter(|plugin| plugin.enabled && plugin.status == "ready")
    .count();

  let status = if !model_ready || !workspace_ready {
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

  HarnessStatusResult {
    status: status.to_string(),
    summary: harness_summary(
      status,
      model_ready,
      workspace_ready,
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
      context_check(&context_window, &output_cap),
      execution_control_check(pending_approval_count, active_turn_count),
      plugin_check(enabled_plugin_count, context.plugins.len()),
      bounded_runtime_check(),
    ],
    metrics: harness_metrics(
      context,
      &model_health.status,
      &model_health.pack_id,
      &context_window,
      enabled_plugin_count,
    ),
  }
}

fn local_model_check(model_ready: bool, display_name: &str, backend: &str) -> HarnessCheck {
  let status = if model_ready {
    "ready"
  } else {
    "setup_required"
  };

  HarnessCheck {
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

fn workspace_check(context: &RuntimeContext) -> HarnessCheck {
  let status = if context.workspace.is_some() {
    "ready"
  } else {
    "setup_required"
  };

  HarnessCheck {
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

fn context_check(context_window: &str, output_cap: &str) -> HarnessCheck {
  HarnessCheck {
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
) -> HarnessCheck {
  HarnessCheck {
    id: "executionControls".to_string(),
    title: "Execution Controls".to_string(),
    status: execution_control_status(pending_approval_count, active_turn_count).to_string(),
    detail: execution_control_detail(pending_approval_count, active_turn_count),
  }
}

fn plugin_check(enabled_plugin_count: usize, plugin_count: usize) -> HarnessCheck {
  let status = if enabled_plugin_count > 0 {
    "ready"
  } else {
    "optional"
  };

  HarnessCheck {
    id: "plugins".to_string(),
    title: "Plugins".to_string(),
    status: status.to_string(),
    detail: format!("{enabled_plugin_count} enabled of {plugin_count} discovered plugin(s)."),
  }
}

fn bounded_runtime_check() -> HarnessCheck {
  HarnessCheck {
    id: "boundedRuntime".to_string(),
    title: "Bounded Runtime".to_string(),
    status: "ready".to_string(),
    detail: "Shell and llama.cpp subprocesses use timeouts and cleanup.".to_string(),
  }
}

fn harness_summary(
  status: &str,
  model_ready: bool,
  workspace_ready: bool,
  pending_approval_count: usize,
  active_turn_count: usize,
) -> String {
  match status {
    "setup_required" if !model_ready => {
      "Download and select one local model to enable the agent harness.".to_string()
    }
    "setup_required" if !workspace_ready => {
      "Open a workspace so tools, memory, and approvals are scoped safely.".to_string()
    }
    "needs_approval" => {
      format!("Harness is waiting on {pending_approval_count} approval(s) before continuing.")
    }
    "running" => format!("Harness is running {active_turn_count} active turn(s)."),
    _ => "Harness ready: model, workspace, tools, context, and plugins are controlled.".to_string(),
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

fn harness_metrics(
  context: &RuntimeContext,
  model_status: &str,
  model_pack_id: &str,
  context_window: &str,
  enabled_plugin_count: usize,
) -> HashMap<String, String> {
  HashMap::from([
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
      "memoryNoteCount".to_string(),
      context.memory_notes.len().to_string(),
    ),
    ("pluginCount".to_string(), context.plugins.len().to_string()),
    (
      "enabledPluginCount".to_string(),
      enabled_plugin_count.to_string(),
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
  ])
}
