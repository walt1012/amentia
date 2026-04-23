use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoragePaths {
  pub database_path: String,
  pub artifacts_path: String,
  pub plugins_path: String,
}

impl StoragePaths {
  pub fn application_support_defaults() -> Self {
    Self {
      database_path: "~/Library/Application Support/Cavell/storage/cavell.db".to_string(),
      artifacts_path: "~/Library/Application Support/Cavell/artifacts".to_string(),
      plugins_path: "~/Library/Application Support/Cavell/plugins".to_string(),
    }
  }
}
