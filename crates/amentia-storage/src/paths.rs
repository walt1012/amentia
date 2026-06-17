use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};

pub(crate) fn default_database_path() -> Result<PathBuf> {
  if let Ok(custom_dir) = env::var("AMENTIA_DATA_DIR") {
    return Ok(PathBuf::from(custom_dir).join("amentia.db"));
  }

  if let Ok(home_dir) = env::var("HOME") {
    return Ok(PathBuf::from(home_dir).join(".amentia").join("amentia.db"));
  }

  if let Ok(home_dir) = env::var("USERPROFILE") {
    return Ok(PathBuf::from(home_dir).join(".amentia").join("amentia.db"));
  }

  Ok(
    env::current_dir()
      .context("failed to read current directory for database path")?
      .join(".amentia")
      .join("amentia.db"),
  )
}

pub(crate) fn default_runtime_state_path() -> Result<PathBuf> {
  if let Ok(custom_dir) = env::var("AMENTIA_DATA_DIR") {
    return Ok(PathBuf::from(custom_dir).join("threads.json"));
  }

  if let Ok(home_dir) = env::var("HOME") {
    return Ok(PathBuf::from(home_dir).join(".amentia").join("threads.json"));
  }

  if let Ok(home_dir) = env::var("USERPROFILE") {
    return Ok(PathBuf::from(home_dir).join(".amentia").join("threads.json"));
  }

  Ok(
    env::current_dir()
      .context("failed to read current directory for runtime state")?
      .join(".amentia")
      .join("threads.json"),
  )
}
