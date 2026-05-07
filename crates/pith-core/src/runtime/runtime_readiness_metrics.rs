use std::collections::HashMap;

use pith_model_runtime::llama_cpp_timeout_seconds;
use pith_sandbox::NativeSandboxStatus;
use pith_tools::shell_command_timeout_seconds;

use crate::runtime_context::RuntimeContext;
use crate::runtime_execution::RuntimeExecutionCounts;

pub(super) struct ReadinessMetricsInput<'a> {
  pub(super) context: &'a RuntimeContext,
  pub(super) model_status: &'a str,
  pub(super) model_pack_id: &'a str,
  pub(super) context_window: &'a str,
  pub(super) enabled_plugin_count: usize,
  pub(super) sandbox_status: &'a NativeSandboxStatus,
  pub(super) workspace_thread_count: usize,
  pub(super) first_request_sent: bool,
  pub(super) execution_counts: RuntimeExecutionCounts,
}

pub(super) fn readiness_metrics(input: ReadinessMetricsInput<'_>) -> HashMap<String, String> {
  let ReadinessMetricsInput {
    context,
    model_status,
    model_pack_id,
    context_window,
    enabled_plugin_count,
    sandbox_status,
    workspace_thread_count,
    first_request_sent,
    execution_counts,
  } = input;

  let mut metrics = HashMap::from([
    ("modelStatus".to_string(), model_status.to_string()),
    ("modelPackId".to_string(), model_pack_id.to_string()),
    (
      "workspaceBound".to_string(),
      context.workspace_state.is_open().to_string(),
    ),
    (
      "pendingApprovalCount".to_string(),
      execution_counts.pending_approval_count().to_string(),
    ),
    (
      "activeTurnCount".to_string(),
      execution_counts.active_turn_count().to_string(),
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
      context.memory_state.note_count().to_string(),
    ),
    (
      "pluginCount".to_string(),
      context.plugin_state.catalog_len().to_string(),
    ),
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
