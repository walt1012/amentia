use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{
  atomic::{AtomicBool, Ordering},
  Arc,
};
use std::time::Duration;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModelRole {
  Default,
  Planner,
  Coder,
  Summarizer,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPackDescriptor {
  pub id: String,
  pub display_name: String,
  pub default_role: ModelRole,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelHealth {
  pub pack_id: String,
  pub display_name: String,
  pub backend: String,
  pub status: String,
  pub detail: String,
  pub source: String,
  pub binary_path: Option<String>,
  pub model_path: Option<String>,
  pub manifest_path: Option<String>,
  pub metrics: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPackManifest {
  pub id: String,
  pub display_name: String,
  pub file_name: String,
  pub context_size: usize,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub model_context_size: Option<usize>,
  pub max_output_tokens: usize,
  pub backend: String,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub license: Option<String>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub homepage: Option<String>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub download_url: Option<String>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub sha256: Option<String>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub size_bytes: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct GenerateRequest {
  pub role: ModelRole,
  pub prompt: String,
  pub max_tokens: usize,
  pub timeout: Option<Duration>,
  pub cancellation: Option<GenerationCancellation>,
}

#[derive(Debug, Clone)]
pub struct GenerateResponse {
  pub text: String,
  pub backend: String,
  pub status: String,
  pub model_id: String,
}

#[derive(Debug, Clone)]
pub struct GenerationCancellation {
  cancelled: Arc<AtomicBool>,
}

impl GenerationCancellation {
  pub fn new() -> Self {
    Self {
      cancelled: Arc::new(AtomicBool::new(false)),
    }
  }

  pub fn cancel(&self) {
    self.cancelled.store(true, Ordering::SeqCst);
  }

  pub fn is_cancelled(&self) -> bool {
    self.cancelled.load(Ordering::SeqCst)
  }
}

impl Default for GenerationCancellation {
  fn default() -> Self {
    Self::new()
  }
}

#[derive(Debug, Clone)]
pub struct ModelBootstrap {
  pub manifest_path: PathBuf,
  pub readme_path: Option<PathBuf>,
  pub copied_files: Vec<PathBuf>,
}

pub fn built_in_model_pack() -> ModelPackDescriptor {
  ModelPackDescriptor {
    id: "granite-4.0-h-350m".to_string(),
    display_name: "Granite 4.0-H-350M".to_string(),
    default_role: ModelRole::Default,
  }
}
