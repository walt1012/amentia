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
  pub output_context: ShellOutputContext,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellSandboxSummary {
  pub mode: String,
  pub backend: String,
  pub active: bool,
  pub temporary_root: Option<String>,
  pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellOutputContext {
  pub mode: String,
  pub source_stdout_bytes: usize,
  pub source_stderr_bytes: usize,
  pub retained_stdout_bytes: usize,
  pub retained_stderr_bytes: usize,
  pub budget_bytes: usize,
  pub was_compacted: bool,
  pub artifact_directory: Option<String>,
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

impl ShellOutputContext {
  pub fn display_line(&self) -> String {
    let artifact = self.artifact_directory.as_deref().unwrap_or("not needed");
    format!(
      "Context: {} retained {}/{} stdout bytes and {}/{} stderr bytes; artifact: {}",
      self.mode,
      self.retained_stdout_bytes,
      self.source_stdout_bytes,
      self.retained_stderr_bytes,
      self.source_stderr_bytes,
      artifact
    )
  }

  pub fn attributes(&self) -> HashMap<String, String> {
    let mut attributes = HashMap::from([
      ("sandboxOutputContextMode".to_string(), self.mode.clone()),
      (
        "sandboxOutputSourceStdoutBytes".to_string(),
        self.source_stdout_bytes.to_string(),
      ),
      (
        "sandboxOutputSourceStderrBytes".to_string(),
        self.source_stderr_bytes.to_string(),
      ),
      (
        "sandboxOutputRetainedStdoutBytes".to_string(),
        self.retained_stdout_bytes.to_string(),
      ),
      (
        "sandboxOutputRetainedStderrBytes".to_string(),
        self.retained_stderr_bytes.to_string(),
      ),
      (
        "sandboxOutputBudgetBytes".to_string(),
        self.budget_bytes.to_string(),
      ),
      (
        "sandboxOutputCompacted".to_string(),
        self.was_compacted.to_string(),
      ),
    ]);
    if let Some(artifact_directory) = &self.artifact_directory {
      attributes.insert(
        "sandboxOutputArtifactDirectory".to_string(),
        artifact_directory.clone(),
      );
    }
    attributes
  }
}
