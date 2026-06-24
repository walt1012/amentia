use std::path::Path;

use amentia_protocol::RuntimeReadinessResult;
use amentia_sandbox::workspace_required_status;
use amentia_tools::{shell_sandbox_status, web_search_status};

use super::runtime_readiness_execution::{bounded_runtime_check, execution_control_check};
use super::runtime_readiness_metrics::{readiness_metrics, ReadinessMetricsInput};
use super::runtime_readiness_model::{context_check, local_model_check};
use super::runtime_readiness_plugins::plugin_check;
use super::runtime_readiness_sandbox::native_sandbox_check;
use super::runtime_readiness_summary::{readiness_summary, ReadinessSummaryInput};
use super::runtime_readiness_web_search::web_search_check;
use super::runtime_readiness_workspace::{
  first_request_check, thread_check, workspace_check,
};
use crate::plugin_permission_sources::{granted_permission_sources, WEB_SEARCH_TOOL_PERMISSION};
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
  let running_workspace_search_count = execution_counts.running_workspace_search_count();
  let sandbox_status = context
    .workspace_state
    .current()
    .map(|workspace| shell_sandbox_status(Path::new(&workspace.root_path)))
    .unwrap_or_else(workspace_required_status);
  let web_search_status = web_search_status();
  let permission_sources = granted_permission_sources(context.plugin_state.catalog());
  let web_search_permission_sources = permission_sources
    .get(WEB_SEARCH_TOOL_PERMISSION)
    .cloned()
    .unwrap_or_default();
  let web_search_permission_ready = !web_search_permission_sources.is_empty();
  let enabled_plugin_count = context.plugin_state.enabled_ready_count();
  let plugin_command_count = context.plugin_state.command_capability_count();
  let enabled_plugin_command_count = context.plugin_state.enabled_command_capability_count();
  let daily_driver = daily_driver_stage(DailyDriverStageInput {
    model_ready,
    workspace_ready,
    thread_ready,
    web_search_permission_ready,
    first_request_sent,
    pending_approval_count,
    active_turn_count,
    running_turn_count,
    running_approval_count,
    running_plugin_command_count,
    running_workspace_search_count,
  });

  let status = readiness_status(ReadinessStatusInput {
    model_ready,
    workspace_ready,
    thread_ready,
    web_search_permission_ready,
    pending_approval_count,
    active_turn_count,
    running_turn_count,
    running_approval_count,
    running_plugin_command_count,
    running_workspace_search_count,
  });
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
      web_search_permission_ready,
      first_request_sent,
      pending_approval_count,
      active_turn_count,
      running_turn_count,
      running_approval_count,
      running_plugin_command_count,
      running_workspace_search_count,
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
        running_workspace_search_count,
      ),
      native_sandbox_check(&sandbox_status),
      web_search_check(&web_search_status, &web_search_permission_sources),
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
      web_search_permission_sources: &web_search_permission_sources,
      workspace_thread_count,
      first_request_sent,
      execution_counts,
      daily_driver_stage: daily_driver.stage,
      daily_driver_next_action: daily_driver.next_action,
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

struct ReadinessStatusInput {
  model_ready: bool,
  workspace_ready: bool,
  thread_ready: bool,
  web_search_permission_ready: bool,
  pending_approval_count: usize,
  active_turn_count: usize,
  running_turn_count: usize,
  running_approval_count: usize,
  running_plugin_command_count: usize,
  running_workspace_search_count: usize,
}

fn readiness_status(input: ReadinessStatusInput) -> &'static str {
  let ReadinessStatusInput {
    model_ready,
    workspace_ready,
    thread_ready,
    web_search_permission_ready,
    pending_approval_count,
    active_turn_count,
    running_turn_count,
    running_approval_count,
    running_plugin_command_count,
    running_workspace_search_count,
  } = input;

  if !model_ready || !workspace_ready || !thread_ready || !web_search_permission_ready {
    "setup_required"
  } else if pending_approval_count > 0 {
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

struct DailyDriverStage {
  stage: &'static str,
  next_action: &'static str,
}

struct DailyDriverStageInput {
  model_ready: bool,
  workspace_ready: bool,
  thread_ready: bool,
  web_search_permission_ready: bool,
  first_request_sent: bool,
  pending_approval_count: usize,
  active_turn_count: usize,
  running_turn_count: usize,
  running_approval_count: usize,
  running_plugin_command_count: usize,
  running_workspace_search_count: usize,
}

fn daily_driver_stage(input: DailyDriverStageInput) -> DailyDriverStage {
  let DailyDriverStageInput {
    model_ready,
    workspace_ready,
    thread_ready,
    web_search_permission_ready,
    first_request_sent,
    pending_approval_count,
    active_turn_count,
    running_turn_count,
    running_approval_count,
    running_plugin_command_count,
    running_workspace_search_count,
  } = input;

  if !model_ready {
    return DailyDriverStage {
      stage: "model_setup",
      next_action: "Download and select a verified local model.",
    };
  }
  if !workspace_ready {
    return DailyDriverStage {
      stage: "workspace_setup",
      next_action: "Open a project to scope tools and memory.",
    };
  }
  if !thread_ready {
    return DailyDriverStage {
      stage: "thread_setup",
      next_action: "Create or select a project-bound session.",
    };
  }
  if pending_approval_count > 0 {
    return DailyDriverStage {
      stage: "approval_review",
      next_action: "Review the pending approval before work continues.",
    };
  }
  if active_turn_count > 0
    || running_turn_count > 0
    || running_approval_count > 0
    || running_plugin_command_count > 0
    || running_workspace_search_count > 0
  {
    return DailyDriverStage {
      stage: "local_execution",
      next_action: "Wait for current work or cancel it if it is no longer useful.",
    };
  }
  if !web_search_permission_ready {
    return DailyDriverStage {
      stage: "retrieval_setup",
      next_action: "Enable Web Search so Amentia can retrieve current information when needed.",
    };
  }
  if !first_request_sent {
    return DailyDriverStage {
      stage: "first_request",
      next_action: "Send the first cowork request.",
    };
  }

  DailyDriverStage {
    stage: "ready",
    next_action: "Ask Amentia for the next cowork task.",
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn status_input() -> ReadinessStatusInput {
    ReadinessStatusInput {
      model_ready: true,
      workspace_ready: true,
      thread_ready: true,
      web_search_permission_ready: true,
      pending_approval_count: 0,
      active_turn_count: 0,
      running_turn_count: 0,
      running_approval_count: 0,
      running_plugin_command_count: 0,
      running_workspace_search_count: 0,
    }
  }

  #[test]
  fn readiness_status_prioritizes_setup_requirements() {
    let status = readiness_status(ReadinessStatusInput {
      model_ready: false,
      pending_approval_count: 1,
      active_turn_count: 1,
      running_turn_count: 1,
      running_approval_count: 1,
      running_plugin_command_count: 1,
      running_workspace_search_count: 1,
      ..status_input()
    });

    assert_eq!(status, "setup_required");
  }

  #[test]
  fn readiness_status_reports_pending_approvals_before_running_work() {
    let status = readiness_status(ReadinessStatusInput {
      pending_approval_count: 1,
      active_turn_count: 1,
      running_turn_count: 1,
      running_approval_count: 1,
      running_plugin_command_count: 1,
      running_workspace_search_count: 1,
      ..status_input()
    });

    assert_eq!(status, "needs_approval");
  }

  #[test]
  fn readiness_status_reports_running_when_work_is_active() {
    let status = readiness_status(ReadinessStatusInput {
      active_turn_count: 1,
      ..status_input()
    });

    assert_eq!(status, "running");
  }

  #[test]
  fn readiness_status_reports_running_when_plugin_command_is_active() {
    let status = readiness_status(ReadinessStatusInput {
      running_plugin_command_count: 1,
      ..status_input()
    });

    assert_eq!(status, "running");
  }

  #[test]
  fn readiness_status_reports_running_when_workspace_search_is_active() {
    let status = readiness_status(ReadinessStatusInput {
      running_workspace_search_count: 1,
      ..status_input()
    });

    assert_eq!(status, "running");
  }

  #[test]
  fn readiness_status_requires_web_search_permission_before_ready() {
    let status = readiness_status(ReadinessStatusInput {
      web_search_permission_ready: false,
      ..status_input()
    });

    assert_eq!(status, "setup_required");
  }

  #[test]
  fn readiness_status_reports_ready_when_setup_is_complete_and_idle() {
    let status = readiness_status(status_input());

    assert_eq!(status, "ready");
  }

  fn stage_input() -> DailyDriverStageInput {
    DailyDriverStageInput {
      model_ready: true,
      workspace_ready: true,
      thread_ready: true,
      web_search_permission_ready: true,
      first_request_sent: true,
      pending_approval_count: 0,
      active_turn_count: 0,
      running_turn_count: 0,
      running_approval_count: 0,
      running_plugin_command_count: 0,
      running_workspace_search_count: 0,
    }
  }

  #[test]
  fn daily_driver_stage_reports_first_actionable_gap() {
    assert_eq!(
      daily_driver_stage(DailyDriverStageInput {
        model_ready: false,
        workspace_ready: false,
        thread_ready: false,
        first_request_sent: false,
        ..stage_input()
      })
      .stage,
      "model_setup"
    );
    assert_eq!(
      daily_driver_stage(DailyDriverStageInput {
        workspace_ready: false,
        thread_ready: false,
        first_request_sent: false,
        ..stage_input()
      })
      .stage,
      "workspace_setup"
    );
    assert_eq!(
      daily_driver_stage(DailyDriverStageInput {
        thread_ready: false,
        first_request_sent: false,
        ..stage_input()
      })
      .stage,
      "thread_setup"
    );
    assert_eq!(
      daily_driver_stage(DailyDriverStageInput {
        web_search_permission_ready: false,
        first_request_sent: false,
        ..stage_input()
      })
      .stage,
      "retrieval_setup"
    );
  }

  #[test]
  fn daily_driver_stage_reports_work_loop_state() {
    assert_eq!(
      daily_driver_stage(DailyDriverStageInput {
        pending_approval_count: 1,
        ..stage_input()
      })
      .stage,
      "approval_review"
    );
    assert_eq!(
      daily_driver_stage(DailyDriverStageInput {
        running_workspace_search_count: 1,
        ..stage_input()
      })
      .stage,
      "local_execution"
    );
    assert_eq!(
      daily_driver_stage(DailyDriverStageInput {
        web_search_permission_ready: false,
        running_workspace_search_count: 1,
        first_request_sent: false,
        ..stage_input()
      })
      .stage,
      "local_execution"
    );
    assert_eq!(
      daily_driver_stage(DailyDriverStageInput {
        first_request_sent: false,
        ..stage_input()
      })
      .stage,
      "first_request"
    );
    assert_eq!(daily_driver_stage(stage_input()).stage, "ready");
  }
}
