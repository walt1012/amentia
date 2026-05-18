use std::path::Path;

use pith_protocol::RuntimeReadinessResult;
use pith_sandbox::workspace_required_status;
use pith_tools::{shell_sandbox_status, web_search_status};

use super::runtime_readiness_checks::{
  bounded_runtime_check, context_check, execution_control_check, first_request_check,
  local_model_check, native_sandbox_check, plugin_check, readiness_summary, thread_check,
  web_search_check, workspace_check, ReadinessSummaryInput,
};
use super::runtime_readiness_metrics::{readiness_metrics, ReadinessMetricsInput};
use crate::runtime_context::RuntimeContext;

pub(crate) fn build_runtime_readiness(context: &RuntimeContext) -> RuntimeReadinessResult {
  let model_health = context.model_state.health();
  let model_ready = model_health.status == "ready";
  let workspace_ready = context.workspace_state.is_open();
  let workspace_thread_count = count_workspace_threads(context);
  let thread_ready = workspace_thread_count > 0;
  let first_request_sent = has_first_request(context);
  let execution_counts = context.execution_state.counts();
  let pending_approval_count = execution_counts.pending_approval_count();
  let active_turn_count = execution_counts.active_turn_count();
  let running_turn_count = execution_counts.running_turn_count();
  let running_approval_count = execution_counts.running_approval_count();
  let running_plugin_command_count = execution_counts.running_plugin_command_count();
  let sandbox_status = context
    .workspace_state
    .current()
    .map(|workspace| shell_sandbox_status(Path::new(&workspace.root_path)))
    .unwrap_or_else(workspace_required_status);
  let web_search_status = web_search_status();
  let enabled_plugin_count = context.plugin_state.enabled_ready_count();
  let plugin_command_count = context.plugin_state.command_capability_count();
  let enabled_plugin_command_count = context.plugin_state.enabled_command_capability_count();

  let status = readiness_status(
    model_ready,
    workspace_ready,
    thread_ready,
    pending_approval_count,
    active_turn_count,
    running_turn_count,
    running_approval_count,
    running_plugin_command_count,
  );
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
    summary: readiness_summary(ReadinessSummaryInput {
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
    }),
    checks: vec![
      local_model_check(
        model_ready,
        &model_health.display_name,
        &model_health.backend,
        &model_health.detail,
      ),
      workspace_check(context),
      thread_check(thread_ready, workspace_thread_count),
      first_request_check(first_request_sent, thread_ready),
      context_check(&context_window, &output_cap),
      execution_control_check(
        pending_approval_count,
        active_turn_count,
        running_turn_count,
        running_approval_count,
        running_plugin_command_count,
      ),
      native_sandbox_check(&sandbox_status),
      web_search_check(&web_search_status),
      plugin_check(
        enabled_plugin_count,
        context.plugin_state.catalog_len(),
        enabled_plugin_command_count,
        plugin_command_count,
      ),
      bounded_runtime_check(),
    ],
    metrics: readiness_metrics(ReadinessMetricsInput {
      context,
      model_status: &model_health.status,
      model_pack_id: &model_health.pack_id,
      context_window: &context_window,
      enabled_plugin_count,
      enabled_plugin_command_count,
      plugin_command_count,
      sandbox_status: &sandbox_status,
      web_search_status: &web_search_status,
      workspace_thread_count,
      first_request_sent,
      execution_counts,
    }),
  }
}

fn count_workspace_threads(context: &RuntimeContext) -> usize {
  context
    .workspace_state
    .current()
    .map(|workspace| context.thread_state.count_for_workspace(workspace))
    .unwrap_or(0)
}

fn has_first_request(context: &RuntimeContext) -> bool {
  context
    .workspace_state
    .current()
    .map(|workspace| {
      context
        .thread_state
        .has_user_message_for_workspace(workspace)
    })
    .unwrap_or(false)
}

fn readiness_status(
  model_ready: bool,
  workspace_ready: bool,
  thread_ready: bool,
  pending_approval_count: usize,
  active_turn_count: usize,
  running_turn_count: usize,
  running_approval_count: usize,
  running_plugin_command_count: usize,
) -> &'static str {
  if !model_ready || !workspace_ready || !thread_ready {
    "setup_required"
  } else if pending_approval_count > 0 {
    "needs_approval"
  } else if active_turn_count > 0
    || running_turn_count > 0
    || running_approval_count > 0
    || running_plugin_command_count > 0
  {
    "running"
  } else {
    "ready"
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn readiness_status_prioritizes_setup_requirements() {
    let status = readiness_status(false, true, true, 1, 1, 1, 1, 1);

    assert_eq!(status, "setup_required");
  }

  #[test]
  fn readiness_status_reports_pending_approvals_before_running_work() {
    let status = readiness_status(true, true, true, 1, 1, 1, 1, 1);

    assert_eq!(status, "needs_approval");
  }

  #[test]
  fn readiness_status_reports_running_when_work_is_active() {
    let status = readiness_status(true, true, true, 0, 1, 0, 0, 0);

    assert_eq!(status, "running");
  }

  #[test]
  fn readiness_status_reports_running_when_plugin_command_is_active() {
    let status = readiness_status(true, true, true, 0, 0, 0, 0, 1);

    assert_eq!(status, "running");
  }

  #[test]
  fn readiness_status_reports_ready_when_setup_is_complete_and_idle() {
    let status = readiness_status(true, true, true, 0, 0, 0, 0, 0);

    assert_eq!(status, "ready");
  }
}
