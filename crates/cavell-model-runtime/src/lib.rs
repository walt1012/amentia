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

pub fn built_in_model_pack() -> ModelPackDescriptor {
  ModelPackDescriptor {
    id: "lfm2.5-350m".to_string(),
    display_name: "LFM2.5-350M".to_string(),
    default_role: ModelRole::Default,
  }
}
