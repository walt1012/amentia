use std::collections::HashMap;

use crate::plugin_permissions::permission_is_granted;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LocalExecutionSafetyMode {
  Explore,
  AskBeforeChange,
  ApprovedWorkspaceExecution,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum LocalChangeExecutionPolicy {
  Denied(LocalChangeBlockReason),
  Ask(String),
  AutoApproved,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LocalChangeBlockReason {
  MissingPermission,
  ReadOnlyMode,
  ApprovalUnavailable,
}

impl LocalExecutionSafetyMode {
  pub(crate) fn from_request(value: Option<&str>) -> Self {
    match value {
      Some("explore") => Self::Explore,
      Some("approvedWorkspaceExecution") => Self::ApprovedWorkspaceExecution,
      _ => Self::AskBeforeChange,
    }
  }

  pub(crate) fn as_str(self) -> &'static str {
    match self {
      Self::Explore => "explore",
      Self::AskBeforeChange => "askBeforeChange",
      Self::ApprovedWorkspaceExecution => "approvedWorkspaceExecution",
    }
  }

  pub(crate) fn change_policy(
    self,
    permission_sources: &HashMap<String, Vec<String>>,
    permission: &str,
    approval_id: Option<String>,
  ) -> LocalChangeExecutionPolicy {
    if !permission_is_granted(permission_sources, permission) {
      return LocalChangeExecutionPolicy::Denied(LocalChangeBlockReason::MissingPermission);
    }

    match self {
      Self::Explore => LocalChangeExecutionPolicy::Denied(LocalChangeBlockReason::ReadOnlyMode),
      Self::AskBeforeChange => approval_id.map(LocalChangeExecutionPolicy::Ask).unwrap_or(
        LocalChangeExecutionPolicy::Denied(LocalChangeBlockReason::ApprovalUnavailable),
      ),
      Self::ApprovedWorkspaceExecution => LocalChangeExecutionPolicy::AutoApproved,
    }
  }

  pub(crate) fn should_reserve_approval_id(
    self,
    permission_sources: &HashMap<String, Vec<String>>,
    permission: &str,
  ) -> bool {
    self == Self::AskBeforeChange && permission_is_granted(permission_sources, permission)
  }
}

impl LocalChangeExecutionPolicy {
  pub(crate) fn approval_policy_attribute(&self) -> &'static str {
    match self {
      Self::Denied(_) => "blocked",
      Self::Ask(_) => "requiresApproval",
      Self::AutoApproved => "autoApproved",
    }
  }

  pub(crate) fn block_reason_attribute(&self) -> Option<&'static str> {
    match self {
      Self::Denied(LocalChangeBlockReason::MissingPermission) => Some("missingPermission"),
      Self::Denied(LocalChangeBlockReason::ReadOnlyMode) => Some("readOnlyMode"),
      Self::Denied(LocalChangeBlockReason::ApprovalUnavailable) => Some("approvalUnavailable"),
      _ => None,
    }
  }

  pub(crate) fn is_missing_permission_denial(&self) -> bool {
    matches!(
      self,
      Self::Denied(LocalChangeBlockReason::MissingPermission)
    )
  }

  pub(crate) fn is_denied(&self) -> bool {
    matches!(self, Self::Denied(_))
  }
}
