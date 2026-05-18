use std::collections::HashMap;

use pith_model_runtime::llama_cpp_timeout_seconds;
use pith_sandbox::NativeSandboxStatus;
use pith_tools::{
  diff_preview_max_bytes, list_directory_max_scanned_entries, search_files_max_file_bytes,
  search_files_max_visited_entries, shell_command_timeout_seconds,
  shell_output_artifact_retained_runs, shell_output_artifact_root_path, web_search_timeout_seconds,
  write_file_max_bytes, WebSearchStatus,
};

use crate::runtime_context::RuntimeContext;
use crate::runtime_execution::RuntimeExecutionCounts;
use crate::turn::turn_tool_limits::{
  LIST_DIRECTORY_RESULT_LIMIT, READ_FILE_PREVIEW_MAX_BYTES, SEARCH_FILES_RESULT_LIMIT,
  SHELL_OUTPUT_PREVIEW_MAX_BYTES, WEB_SEARCH_RESULT_LIMIT,
};

pub(super) struct ReadinessMetricsInput<'a> {
  pub(super) context: &'a RuntimeContext,
  pub(super) model_status: &'a str,
  pub(super) model_pack_id: &'a str,
  pub(super) context_window: &'a str,
  pub(super) enabled_plugin_count: usize,
  pub(super) enabled_plugin_command_count: usize,
  pub(super) plugin_command_count: usize,
  pub(super) sandbox_status: &'a NativeSandboxStatus,
  pub(super) web_search_status: &'a WebSearchStatus,
  pub(super) web_search_permission_sources: &'a [String],
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
    enabled_plugin_command_count,
    plugin_command_count,
    sandbox_status,
    web_search_status,
    web_search_permission_sources,
    workspace_thread_count,
    first_request_sent,
    execution_counts,
  } = input;

  let mut metrics = HashMap::new();
  insert_model_metrics(&mut metrics, model_status, model_pack_id, context_window);
  insert_workspace_metrics(
    &mut metrics,
    context,
    workspace_thread_count,
    first_request_sent,
  );
  insert_execution_metrics(&mut metrics, execution_counts);
  insert_memory_metrics(&mut metrics, context);
  insert_plugin_metrics(
    &mut metrics,
    context,
    enabled_plugin_count,
    enabled_plugin_command_count,
    plugin_command_count,
  );
  insert_sandbox_metrics(&mut metrics, sandbox_status);
  insert_tool_limit_metrics(&mut metrics);
  insert_web_search_metrics(
    &mut metrics,
    web_search_status,
    web_search_permission_sources,
  );
  metrics
}

fn insert_model_metrics(
  metrics: &mut HashMap<String, String>,
  model_status: &str,
  model_pack_id: &str,
  context_window: &str,
) {
  insert_metric(metrics, "modelStatus", model_status);
  insert_metric(metrics, "modelPackId", model_pack_id);
  insert_metric(metrics, "contextWindowTokens", context_window);
  insert_metric(
    metrics,
    "llamaTimeoutSeconds",
    llama_cpp_timeout_seconds().to_string(),
  );
}

fn insert_workspace_metrics(
  metrics: &mut HashMap<String, String>,
  context: &RuntimeContext,
  workspace_thread_count: usize,
  first_request_sent: bool,
) {
  insert_metric(
    metrics,
    "workspaceBound",
    context.workspace_state.is_open().to_string(),
  );
  insert_metric(
    metrics,
    "workspaceThreadCount",
    workspace_thread_count.to_string(),
  );
  insert_metric(metrics, "firstRequestSent", first_request_sent.to_string());
}

fn insert_execution_metrics(
  metrics: &mut HashMap<String, String>,
  execution_counts: RuntimeExecutionCounts,
) {
  insert_metric(
    metrics,
    "pendingApprovalCount",
    execution_counts.pending_approval_count().to_string(),
  );
  insert_metric(
    metrics,
    "activeTurnCount",
    execution_counts.active_turn_count().to_string(),
  );
  insert_metric(
    metrics,
    "runningTurnCount",
    execution_counts.running_turn_count().to_string(),
  );
  insert_metric(
    metrics,
    "runningApprovalCount",
    execution_counts.running_approval_count().to_string(),
  );
  insert_metric(
    metrics,
    "runningPluginCommandCount",
    execution_counts.running_plugin_command_count().to_string(),
  );
}

fn insert_memory_metrics(metrics: &mut HashMap<String, String>, context: &RuntimeContext) {
  insert_metric(
    metrics,
    "memoryNoteCount",
    context.memory_state.note_count().to_string(),
  );
}

fn insert_plugin_metrics(
  metrics: &mut HashMap<String, String>,
  context: &RuntimeContext,
  enabled_plugin_count: usize,
  enabled_plugin_command_count: usize,
  plugin_command_count: usize,
) {
  insert_metric(
    metrics,
    "pluginCount",
    context.plugin_state.catalog_len().to_string(),
  );
  insert_metric(
    metrics,
    "enabledPluginCount",
    enabled_plugin_count.to_string(),
  );
  insert_metric(
    metrics,
    "pluginCommandCount",
    plugin_command_count.to_string(),
  );
  insert_metric(
    metrics,
    "enabledPluginCommandCount",
    enabled_plugin_command_count.to_string(),
  );
  insert_metric(
    metrics,
    "pluginRootCount",
    context.plugin_state.roots().len().to_string(),
  );
  insert_metric(
    metrics,
    "pluginRoots",
    context
      .plugin_state
      .roots()
      .iter()
      .map(|root| root.display().to_string())
      .collect::<Vec<_>>()
      .join("\n"),
  );
  insert_metric(
    metrics,
    "pluginInstallRoot",
    context.plugin_state.install_root().display().to_string(),
  );
}

fn insert_sandbox_metrics(
  metrics: &mut HashMap<String, String>,
  sandbox_status: &NativeSandboxStatus,
) {
  insert_metric(metrics, "sandboxMode", sandbox_status.mode.clone());
  insert_metric(metrics, "sandboxBackend", sandbox_status.backend.clone());
  insert_metric(
    metrics,
    "sandboxAvailable",
    sandbox_status.available.to_string(),
  );
  insert_metric(metrics, "sandboxActive", sandbox_status.active.to_string());
  insert_metric(
    metrics,
    "sandboxNetworkAllowed",
    sandbox_status.network_allowed.to_string(),
  );
  insert_metric(
    metrics,
    "sandboxNetworkPolicy",
    sandbox_status.network_policy(),
  );
  if let Some(temporary_root) = &sandbox_status.temporary_root {
    insert_metric(metrics, "sandboxTempRoot", temporary_root.clone());
  }
  if !sandbox_status.writable_roots.is_empty() {
    insert_metric(
      metrics,
      "sandboxWritableRoots",
      sandbox_status.writable_roots.join("\n"),
    );
  }
}

fn insert_tool_limit_metrics(metrics: &mut HashMap<String, String>) {
  insert_metric(
    metrics,
    "shellTimeoutSeconds",
    shell_command_timeout_seconds().to_string(),
  );
  insert_metric(
    metrics,
    "shellOutputArtifactRoot",
    shell_output_artifact_root_path(),
  );
  insert_metric(
    metrics,
    "shellOutputArtifactRetainedRuns",
    shell_output_artifact_retained_runs().to_string(),
  );
  insert_metric(
    metrics,
    "workspaceSearchMaxFileBytes",
    search_files_max_file_bytes().to_string(),
  );
  insert_metric(
    metrics,
    "workspaceSearchMaxVisitedEntries",
    search_files_max_visited_entries().to_string(),
  );
  insert_metric(
    metrics,
    "directoryListingMaxScannedEntries",
    list_directory_max_scanned_entries().to_string(),
  );
  insert_metric(
    metrics,
    "diffPreviewMaxBytes",
    diff_preview_max_bytes().to_string(),
  );
  insert_metric(
    metrics,
    "workspaceWriteMaxBytes",
    write_file_max_bytes().to_string(),
  );
  insert_metric(
    metrics,
    "turnReadFileMaxBytes",
    READ_FILE_PREVIEW_MAX_BYTES.to_string(),
  );
  insert_metric(
    metrics,
    "turnListDirectoryMaxResults",
    LIST_DIRECTORY_RESULT_LIMIT.to_string(),
  );
  insert_metric(
    metrics,
    "turnSearchFilesMaxResults",
    SEARCH_FILES_RESULT_LIMIT.to_string(),
  );
  insert_metric(
    metrics,
    "turnShellOutputMaxBytes",
    SHELL_OUTPUT_PREVIEW_MAX_BYTES.to_string(),
  );
  insert_metric(
    metrics,
    "turnWebSearchMaxResults",
    WEB_SEARCH_RESULT_LIMIT.to_string(),
  );
}

fn insert_web_search_metrics(
  metrics: &mut HashMap<String, String>,
  web_search_status: &WebSearchStatus,
  permission_sources: &[String],
) {
  insert_metric(
    metrics,
    "webSearchTimeoutSeconds",
    web_search_timeout_seconds().to_string(),
  );
  insert_metric(
    metrics,
    "webSearchProvider",
    web_search_status.provider.clone(),
  );
  insert_metric(metrics, "webSearchClient", web_search_status.client.clone());
  insert_metric(
    metrics,
    "webSearchAvailable",
    web_search_status.available.to_string(),
  );
  insert_metric(
    metrics,
    "webSearchPermissionGranted",
    (!permission_sources.is_empty()).to_string(),
  );
  insert_metric(
    metrics,
    "webSearchPermissionSources",
    permission_sources.join(", "),
  );
}

fn insert_metric(metrics: &mut HashMap<String, String>, key: &str, value: impl Into<String>) {
  metrics.insert(key.to_string(), value.into());
}
