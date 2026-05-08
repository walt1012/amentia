use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BuiltInTool {
  ReadFile,
  WriteFile,
  ListDirectory,
  SearchFiles,
  WebSearch,
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
pub struct WebSearchResult {
  pub title: String,
  pub url: String,
  pub snippet: String,
  pub source: String,
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
  pub network_allowed: bool,
  pub temporary_root: Option<String>,
  pub writable_roots: Vec<String>,
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
    let network_state = if self.network_allowed {
      "network allowed"
    } else {
      "network denied"
    };
    format!(
      "Sandbox: {} via {} ({}, {})",
      self.mode,
      self.backend,
      self.state(),
      network_state
    )
  }

  pub fn attributes(&self) -> HashMap<String, String> {
    let mut attributes = HashMap::from([
      ("sandboxMode".to_string(), self.mode.clone()),
      ("sandboxBackend".to_string(), self.backend.clone()),
      ("sandboxActive".to_string(), self.active.to_string()),
      (
        "sandboxNetworkAllowed".to_string(),
        self.network_allowed.to_string(),
      ),
      ("sandboxDetail".to_string(), self.detail.clone()),
    ]);
    if let Some(temporary_root) = &self.temporary_root {
      attributes.insert("sandboxTempRoot".to_string(), temporary_root.clone());
    }
    if !self.writable_roots.is_empty() {
      attributes.insert(
        "sandboxWritableRoots".to_string(),
        self.writable_roots.join("\n"),
      );
    }
    attributes
  }
}

impl ShellOutputContext {
  pub fn stdout_artifact_path(&self) -> Option<String> {
    self
      .artifact_directory
      .as_deref()
      .map(|directory| Path::new(directory).join("stdout.txt").display().to_string())
  }

  pub fn stderr_artifact_path(&self) -> Option<String> {
    self
      .artifact_directory
      .as_deref()
      .map(|directory| Path::new(directory).join("stderr.txt").display().to_string())
  }

  pub fn source_total_bytes(&self) -> usize {
    self
      .source_stdout_bytes
      .saturating_add(self.source_stderr_bytes)
  }

  pub fn retained_total_bytes(&self) -> usize {
    self
      .retained_stdout_bytes
      .saturating_add(self.retained_stderr_bytes)
  }

  pub fn saved_bytes(&self) -> usize {
    self
      .source_total_bytes()
      .saturating_sub(self.retained_total_bytes())
  }

  pub fn savings_percent(&self) -> usize {
    let source_total = self.source_total_bytes();
    if source_total == 0 {
      return 0;
    }

    self.saved_bytes().saturating_mul(100) / source_total
  }

  pub fn display_line(&self) -> String {
    let artifact = match (self.stdout_artifact_path(), self.stderr_artifact_path()) {
      (Some(stdout_path), Some(stderr_path)) => {
        format!("stdout {stdout_path}, stderr {stderr_path}")
      }
      _ => "not needed".to_string(),
    };
    format!(
      "Context: {} retained {}/{} stdout bytes and {}/{} stderr bytes; saved {}%; artifacts: {}",
      self.mode,
      self.retained_stdout_bytes,
      self.source_stdout_bytes,
      self.retained_stderr_bytes,
      self.source_stderr_bytes,
      self.savings_percent(),
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
      (
        "sandboxOutputSavedBytes".to_string(),
        self.saved_bytes().to_string(),
      ),
      (
        "sandboxOutputSavingsPercent".to_string(),
        self.savings_percent().to_string(),
      ),
    ]);
    if let Some(artifact_directory) = &self.artifact_directory {
      attributes.insert(
        "sandboxOutputArtifactDirectory".to_string(),
        artifact_directory.clone(),
      );
      if let Some(stdout_path) = self.stdout_artifact_path() {
        attributes.insert("sandboxOutputStdoutArtifactPath".to_string(), stdout_path);
      }
      if let Some(stderr_path) = self.stderr_artifact_path() {
        attributes.insert("sandboxOutputStderrArtifactPath".to_string(), stderr_path);
      }
    }
    attributes
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn shell_output_context_reports_context_savings() {
    let context = ShellOutputContext {
      mode: "sandboxOutputPreview".to_string(),
      source_stdout_bytes: 900,
      source_stderr_bytes: 100,
      retained_stdout_bytes: 100,
      retained_stderr_bytes: 50,
      budget_bytes: 256,
      was_compacted: true,
      artifact_directory: Some("/tmp/pith/run-1".to_string()),
    };

    assert_eq!(context.source_total_bytes(), 1000);
    assert_eq!(context.retained_total_bytes(), 150);
    assert_eq!(context.saved_bytes(), 850);
    assert_eq!(context.savings_percent(), 85);
    assert!(context.display_line().contains("saved 85%"));
    assert!(context.display_line().contains("stdout /tmp/pith/run-1/stdout.txt"));
    assert_eq!(
      context.stdout_artifact_path().as_deref(),
      Some("/tmp/pith/run-1/stdout.txt")
    );
    assert_eq!(
      context.stderr_artifact_path().as_deref(),
      Some("/tmp/pith/run-1/stderr.txt")
    );
    assert_eq!(context.attributes()["sandboxOutputSavingsPercent"], "85");
    assert_eq!(
      context.attributes()["sandboxOutputStdoutArtifactPath"],
      "/tmp/pith/run-1/stdout.txt"
    );
  }
}
