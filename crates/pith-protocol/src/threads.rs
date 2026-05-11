use crate::workspace::WorkspaceSummary;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadSummary {
  pub id: String,
  pub title: String,
  pub status: String,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub workspace: Option<WorkspaceSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadListResult {
  pub threads: Vec<ThreadSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadStartParams {
  pub title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadStartResult {
  pub thread: ThreadSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadReadParams {
  pub thread_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TurnStartParams {
  pub thread_id: String,
  pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineItem {
  pub kind: String,
  pub title: String,
  pub content: String,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub attributes: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApprovalRequest {
  pub id: String,
  pub thread_id: String,
  pub action: String,
  pub title: String,
  pub relative_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApprovalRespondParams {
  pub approval_id: String,
  pub decision: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadReadResult {
  pub thread: ThreadSummary,
  pub items: Vec<TimelineItem>,
  pub pending_approvals: Vec<ApprovalRequest>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub active_turn_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadUpdatedNotificationParams {
  pub thread: ThreadSummary,
  pub items: Vec<TimelineItem>,
  pub pending_approvals: Vec<ApprovalRequest>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub active_turn_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TurnStartResult {
  pub turn_id: String,
  pub thread_id: String,
  pub items: Vec<TimelineItem>,
  pub pending_approvals: Vec<ApprovalRequest>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub active_turn_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TurnCancelParams {
  pub turn_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TurnCancelRunningParams {
  pub thread_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TurnCancelResult {
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub turn_id: Option<String>,
  pub thread_id: String,
  pub items: Vec<TimelineItem>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub active_turn_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApprovalRespondResult {
  pub approval_id: String,
  pub thread_id: String,
  pub items: Vec<TimelineItem>,
  pub pending_approvals: Vec<ApprovalRequest>,
}
