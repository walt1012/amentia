use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use rusqlite::Connection;

mod approvals;
mod legacy;
mod memory;
mod paths;
mod plugins;
mod schema;
#[cfg(test)]
mod tests;
mod threads;
mod time;
mod types;
mod workspace;

use crate::paths::{default_database_path, default_runtime_state_path};
use crate::schema::ensure_schema;
pub use crate::types::{
  StoragePaths, StoredApprovalRecord, StoredPluginConnectorCredential, StoredThreadRecord,
};

#[derive(Debug, Clone)]
pub struct RuntimeStore {
  database_path: PathBuf,
  legacy_runtime_state_path: PathBuf,
}

impl RuntimeStore {
  pub fn new_default() -> Result<Self> {
    Ok(Self {
      database_path: default_database_path()?,
      legacy_runtime_state_path: default_runtime_state_path()?,
    })
  }

  pub fn new(database_path: PathBuf, legacy_runtime_state_path: PathBuf) -> Self {
    Self {
      database_path,
      legacy_runtime_state_path,
    }
  }

  fn open_connection(&self) -> Result<Connection> {
    if let Some(parent) = self.database_path.parent() {
      fs::create_dir_all(parent)
        .with_context(|| format!("failed to create storage directory {}", parent.display()))?;
    }

    let connection = Connection::open(&self.database_path)
      .with_context(|| format!("failed to open database {}", self.database_path.display()))?;
    ensure_schema(&connection)?;
    Ok(connection)
  }
}
