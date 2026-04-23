use std::env;
use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use cavell_protocol::{ThreadSummary, TimelineItem};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoragePaths {
  pub database_path: String,
  pub artifacts_path: String,
  pub plugins_path: String,
  pub runtime_state_path: String,
}

impl StoragePaths {
  pub fn application_support_defaults() -> Self {
    Self {
      database_path: "~/Library/Application Support/Cavell/storage/cavell.db".to_string(),
      artifacts_path: "~/Library/Application Support/Cavell/artifacts".to_string(),
      plugins_path: "~/Library/Application Support/Cavell/plugins".to_string(),
      runtime_state_path: "~/Library/Application Support/Cavell/storage/threads.json".to_string(),
    }
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredThreadRecord {
  pub summary: ThreadSummary,
  pub turn_count: usize,
  pub items: Vec<TimelineItem>,
}

#[derive(Debug, Clone)]
pub struct FileThreadStore {
  path: PathBuf,
}

impl FileThreadStore {
  pub fn new_default() -> Result<Self> {
    Ok(Self {
      path: default_runtime_state_path()?,
    })
  }

  pub fn load_threads(&self) -> Result<Vec<StoredThreadRecord>> {
    if !self.path.exists() {
      return Ok(vec![]);
    }

    let content = fs::read_to_string(&self.path)
      .with_context(|| format!("failed to read runtime state from {}", self.path.display()))?;

    let threads = serde_json::from_str::<Vec<StoredThreadRecord>>(&content)
      .with_context(|| format!("failed to parse runtime state from {}", self.path.display()))?;

    Ok(threads)
  }

  pub fn save_threads(&self, threads: &[StoredThreadRecord]) -> Result<()> {
    if let Some(parent) = self.path.parent() {
      fs::create_dir_all(parent).with_context(|| {
        format!(
          "failed to create runtime state directory {}",
          parent.display()
        )
      })?;
    }

    let content =
      serde_json::to_string_pretty(threads).context("failed to serialize runtime thread state")?;
    fs::write(&self.path, content)
      .with_context(|| format!("failed to write runtime state to {}", self.path.display()))?;

    Ok(())
  }
}

fn default_runtime_state_path() -> Result<PathBuf> {
  if let Ok(custom_dir) = env::var("CAVELL_DATA_DIR") {
    return Ok(PathBuf::from(custom_dir).join("threads.json"));
  }

  if let Ok(home_dir) = env::var("HOME") {
    return Ok(PathBuf::from(home_dir).join(".cavell").join("threads.json"));
  }

  if let Ok(home_dir) = env::var("USERPROFILE") {
    return Ok(PathBuf::from(home_dir).join(".cavell").join("threads.json"));
  }

  Ok(
    env::current_dir()
      .context("failed to read current directory for runtime state")?
      .join(".cavell")
      .join("threads.json"),
  )
}
