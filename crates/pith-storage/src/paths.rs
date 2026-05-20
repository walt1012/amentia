use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};

pub(crate) fn default_database_path() -> Result<PathBuf> {
  if let Ok(custom_dir) = env::var("PITH_DATA_DIR") {
    return Ok(PathBuf::from(custom_dir).join("pith.db"));
  }

  if let Ok(home_dir) = env::var("HOME") {
    return Ok(PathBuf::from(home_dir).join(".pith").join("pith.db"));
  }

  if let Ok(home_dir) = env::var("USERPROFILE") {
    return Ok(PathBuf::from(home_dir).join(".pith").join("pith.db"));
  }

  Ok(
    env::current_dir()
      .context("failed to read current directory for database path")?
      .join(".pith")
      .join("pith.db"),
  )
}

pub(crate) fn default_runtime_state_path() -> Result<PathBuf> {
  if let Ok(custom_dir) = env::var("PITH_DATA_DIR") {
    return Ok(PathBuf::from(custom_dir).join("threads.json"));
  }

  if let Ok(home_dir) = env::var("HOME") {
    return Ok(PathBuf::from(home_dir).join(".pith").join("threads.json"));
  }

  if let Ok(home_dir) = env::var("USERPROFILE") {
    return Ok(PathBuf::from(home_dir).join(".pith").join("threads.json"));
  }

  Ok(
    env::current_dir()
      .context("failed to read current directory for runtime state")?
      .join(".pith")
      .join("threads.json"),
  )
}
