use std::fmt;

use pith_memory::MemoryNote;
use pith_model_runtime::GenerationCancellation;
use pith_plugin_host::PluginCommandEntry as HostPluginCommandEntry;
use pith_protocol::{TimelineItem, WorkspaceSummary};
use serde::Serialize;

use crate::approval_types::PendingApproval;

pub(super) const NO_CREDENTIAL_PROVIDER: &str = "pith.noCredentialRequired";

#[derive(Debug)]
pub struct PreparedPluginCommandRun {
  pub(super) request_id: serde_json::Value,
  pub(super) snapshot: PluginCommandSnapshot,
}

#[derive(Debug)]
pub struct CompletedPluginCommandRun {
  pub(super) request_id: serde_json::Value,
  pub(super) running_id: String,
  pub(super) output: std::result::Result<PluginCommandOutput, (i32, String)>,
}

#[derive(Debug)]
pub(crate) struct PluginCommandSnapshot {
  pub(super) thread_id: String,
  pub(super) command: HostPluginCommandEntry,
  pub(super) workspace: Option<WorkspaceSummary>,
  pub(super) input: Option<String>,
  pub(super) connector_refs: Vec<PluginConnectorExecutionRef>,
  pub(super) command_item: TimelineItem,
  pub(super) memory_notes: Vec<MemoryNote>,
  pub(super) cancellation: GenerationCancellation,
  pub(super) running_id: String,
  pub(super) approval_id: Option<String>,
}

impl PluginCommandSnapshot {
  pub(crate) fn command_id(&self) -> &str {
    &self.command.command_id
  }
}

#[derive(Debug, Clone)]
pub(crate) struct PluginRunnerMemoryNoteDraft {
  pub(super) title: String,
  pub(super) body: String,
  pub(super) source: Option<String>,
  pub(super) tags: Vec<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct PluginConnectorExecutionRef {
  pub(super) connector_id: String,
  pub(super) service: String,
  pub(super) credential_provider: PluginConnectorCredentialProviderRef,
  #[serde(skip)]
  pub(super) credential_secret: Option<String>,
}

impl fmt::Debug for PluginConnectorExecutionRef {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    formatter
      .debug_struct("PluginConnectorExecutionRef")
      .field("connector_id", &self.connector_id)
      .field("service", &self.service)
      .field("credential_provider", &self.credential_provider)
      .field(
        "credential_secret",
        &self.credential_secret.as_ref().map(|_| "<redacted>"),
      )
      .finish()
  }
}

impl PluginConnectorExecutionRef {
  pub(super) fn credential_binding(&self) -> &'static str {
    if self.credential_provider.provider == NO_CREDENTIAL_PROVIDER {
      return "none";
    }
    if self.credential_provider.env_key.is_some() {
      return "env-bound";
    }

    "marker-only"
  }

  pub(super) fn requires_user_approval(&self) -> bool {
    self.credential_provider.provider != NO_CREDENTIAL_PROVIDER
  }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct PluginConnectorCredentialProviderRef {
  pub(super) provider: String,
  pub(super) handle: String,
  pub(super) store: String,
  pub(super) label: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub(super) env_key: Option<String>,
  pub(super) authorized_at: i64,
}

#[derive(Debug)]
pub(crate) struct PluginCommandOutput {
  pub(crate) thread_id: String,
  pub(crate) command: HostPluginCommandEntry,
  pub(crate) workspace: Option<WorkspaceSummary>,
  pub(crate) input: Option<String>,
  pub(crate) items: Vec<TimelineItem>,
  pub(crate) capture_memory: bool,
  pub(crate) runner_memory_notes: Vec<PluginRunnerMemoryNoteDraft>,
  pub(crate) pending_approval: Option<PendingApproval>,
}
