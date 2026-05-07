use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BuiltInTool {
  ReadFile,
  WriteFile,
  ListDirectory,
  SearchFiles,
  RunShell,
  GenerateDiff,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectoryEntry {
  pub name: String,
  pub relative_path: String,
  pub entry_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadFileResult {
  pub relative_path: String,
  pub content: String,
  pub is_truncated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchMatch {
  pub relative_path: String,
  pub line_number: usize,
  pub line: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellCommandResult {
  pub command: String,
  pub exit_code: i32,
  pub stdout: String,
  pub stderr: String,
  pub was_truncated: bool,
  pub timed_out: bool,
  pub sandbox: ShellSandboxSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellSandboxSummary {
  pub mode: String,
  pub backend: String,
  pub active: bool,
  pub temporary_root: Option<String>,
  pub detail: String,
}

impl ShellSandboxSummary {
  pub fn state(&self) -> &'static str {
    if self.active {
      "active"
    } else {
      "limited"
    }
  }

  pub fn display_line(&self) -> String {
    format!(
      "Sandbox: {} via {} ({})",
      self.mode,
      self.backend,
      self.state()
    )
  }

  pub fn attributes(&self) -> HashMap<String, String> {
    let mut attributes = HashMap::from([
      ("sandboxMode".to_string(), self.mode.clone()),
      ("sandboxBackend".to_string(), self.backend.clone()),
      ("sandboxActive".to_string(), self.active.to_string()),
      ("sandboxDetail".to_string(), self.detail.clone()),
    ]);
    if let Some(temporary_root) = &self.temporary_root {
      attributes.insert("sandboxTempRoot".to_string(), temporary_root.clone());
    }
    attributes
  }
}
