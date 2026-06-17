use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryNoteSummary {
  pub id: String,
  pub title: String,
  pub body: String,
  pub scope: String,
  pub source: String,
  pub created_at: i64,
  pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryStatusResult {
  pub note_count: usize,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub latest_title: Option<String>,
  pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryListResult {
  pub notes: Vec<MemoryNoteSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryCreateParams {
  pub title: String,
  pub body: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryCreateResult {
  pub note: MemoryNoteSummary,
}
