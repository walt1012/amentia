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
};

pub(super) struct ReadinessMetricsInput<'a> {
  pub(super) context: &'a RuntimeContext,
  pub(super) model_status: &'a str,
  pub(super) model_pack_id: &'a str,
  pub(super) context_window: &'a str,
  pub(super) enabled_plugin_count: usize,
  pub(super) sandbox_status: &'a NativeSandboxStatus,
  pub(super) web_search_status: &'a WebSearchStatus,
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
    web_search_status,
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
      "sandboxNetworkAllowed".to_string(),
      sandbox_status.network_allowed.to_string(),
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
      "shellOutputArtifactRoot".to_string(),
      shell_output_artifact_root_path(),
    ),
    (
      "shellOutputArtifactRetainedRuns".to_string(),
      shell_output_artifact_retained_runs().to_string(),
    ),
    (
      "workspaceSearchMaxFileBytes".to_string(),
      search_files_max_file_bytes().to_string(),
    ),
    (
      "workspaceSearchMaxVisitedEntries".to_string(),
      search_files_max_visited_entries().to_string(),
    ),
    (
      "directoryListingMaxScannedEntries".to_string(),
      list_directory_max_scanned_entries().to_string(),
    ),
    (
      "diffPreviewMaxBytes".to_string(),
      diff_preview_max_bytes().to_string(),
    ),
    (
      "workspaceWriteMaxBytes".to_string(),
      write_file_max_bytes().to_string(),
    ),
    (
      "turnReadFileMaxBytes".to_string(),
      READ_FILE_PREVIEW_MAX_BYTES.to_string(),
    ),
    (
      "turnListDirectoryMaxResults".to_string(),
      LIST_DIRECTORY_RESULT_LIMIT.to_string(),
    ),
    (
      "turnSearchFilesMaxResults".to_string(),
      SEARCH_FILES_RESULT_LIMIT.to_string(),
    ),
    (
      "webSearchTimeoutSeconds".to_string(),
      web_search_timeout_seconds().to_string(),
    ),
    (
      "webSearchProvider".to_string(),
      web_search_status.provider.clone(),
    ),
    (
      "webSearchClient".to_string(),
      web_search_status.client.clone(),
    ),
    (
      "webSearchAvailable".to_string(),
      web_search_status.available.to_string(),
    ),
    (
      "llamaTimeoutSeconds".to_string(),
      llama_cpp_timeout_seconds().to_string(),
    ),
  ]);
  if let Some(temporary_root) = &sandbox_status.temporary_root {
    metrics.insert("sandboxTempRoot".to_string(), temporary_root.clone());
  }
  if !sandbox_status.writable_roots.is_empty() {
    metrics.insert(
      "sandboxWritableRoots".to_string(),
      sandbox_status.writable_roots.join("\n"),
    );
  }
  metrics
}
