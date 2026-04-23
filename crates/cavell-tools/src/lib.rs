use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{bail, Context, Result};
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
}

pub fn list_directory(
  workspace_root: &Path,
  relative_path: Option<&str>,
  limit: usize,
) -> Result<Vec<DirectoryEntry>> {
  let target = resolve_workspace_path(workspace_root, relative_path.unwrap_or("."), true)?;
  let workspace_root = fs::canonicalize(workspace_root).with_context(|| {
    format!(
      "failed to resolve workspace root {}",
      workspace_root.display()
    )
  })?;

  let mut entries = fs::read_dir(&target)
    .with_context(|| format!("failed to read directory {}", target.display()))?
    .filter_map(|entry| entry.ok())
    .map(|entry| {
      let path = entry.path();
      let metadata = entry
        .metadata()
        .with_context(|| format!("failed to read metadata for {}", path.display()))?;
      let entry_type = if metadata.is_dir() {
        "directory"
      } else {
        "file"
      };
      let relative_path = relative_path_string(&workspace_root, &path)?;

      Ok(DirectoryEntry {
        name: entry.file_name().to_string_lossy().into_owned(),
        relative_path,
        entry_type: entry_type.to_string(),
      })
    })
    .collect::<Result<Vec<_>>>()?;

  entries.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
  if entries.len() > limit {
    entries.truncate(limit);
  }

  Ok(entries)
}

pub fn read_file(
  workspace_root: &Path,
  relative_path: &str,
  max_bytes: usize,
) -> Result<ReadFileResult> {
  let target = resolve_workspace_path(workspace_root, relative_path, false)?;
  let workspace_root = fs::canonicalize(workspace_root).with_context(|| {
    format!(
      "failed to resolve workspace root {}",
      workspace_root.display()
    )
  })?;
  let bytes =
    fs::read(&target).with_context(|| format!("failed to read file {}", target.display()))?;
  let is_truncated = bytes.len() > max_bytes;
  let preview_bytes = if is_truncated {
    &bytes[..max_bytes]
  } else {
    &bytes[..]
  };

  Ok(ReadFileResult {
    relative_path: relative_path_string(&workspace_root, &target)?,
    content: String::from_utf8_lossy(preview_bytes).into_owned(),
    is_truncated,
  })
}

pub fn write_file(workspace_root: &Path, relative_path: &str, content: &str) -> Result<String> {
  let workspace_root = canonical_workspace_root(workspace_root)?;
  let sanitized_relative_path = sanitize_relative_path(relative_path)?;
  let target = workspace_root.join(&sanitized_relative_path);

  if let Some(parent) = target.parent() {
    fs::create_dir_all(parent)
      .with_context(|| format!("failed to create directory {}", parent.display()))?;
  }

  if target.is_dir() {
    bail!("workspace path points to a directory");
  }

  fs::write(&target, content)
    .with_context(|| format!("failed to write file {}", target.display()))?;

  Ok(sanitized_relative_path)
}

pub fn search_files(
  workspace_root: &Path,
  query: &str,
  max_results: usize,
) -> Result<Vec<SearchMatch>> {
  let workspace_root = canonical_workspace_root(workspace_root)?;
  let normalized_query = query.trim().to_lowercase();

  if normalized_query.is_empty() {
    bail!("search query must not be empty");
  }

  let mut results = vec![];
  visit_directory(
    &workspace_root,
    &workspace_root,
    &normalized_query,
    max_results,
    &mut results,
  )?;

  Ok(results)
}

pub fn run_shell(
  workspace_root: &Path,
  command: &str,
  max_output_bytes: usize,
) -> Result<ShellCommandResult> {
  let workspace_root = canonical_workspace_root(workspace_root)?;
  let trimmed_command = command.trim();
  if trimmed_command.is_empty() {
    bail!("shell command must not be empty");
  }

  let output = build_shell_command(trimmed_command)
    .current_dir(&workspace_root)
    .output()
    .with_context(|| {
      format!(
        "failed to run shell command in {}",
        workspace_root.display()
      )
    })?;

  let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
  let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
  let combined_len = stdout.len() + stderr.len();
  let was_truncated = combined_len > max_output_bytes * 2;

  Ok(ShellCommandResult {
    command: trimmed_command.to_string(),
    exit_code: output.status.code().unwrap_or(-1),
    stdout: truncate_output(&stdout, max_output_bytes),
    stderr: truncate_output(&stderr, max_output_bytes),
    was_truncated,
  })
}

pub fn generate_diff(
  workspace_root: &Path,
  relative_path: &str,
  next_content: &str,
) -> Result<String> {
  let workspace_root = canonical_workspace_root(workspace_root)?;
  let sanitized_relative_path = sanitize_relative_path(relative_path)?;
  let target = workspace_root.join(&sanitized_relative_path);

  if target.is_dir() {
    bail!("workspace path points to a directory");
  }

  let previous_content = if target.is_file() {
    fs::read_to_string(&target)
      .with_context(|| format!("failed to read file {}", target.display()))?
  } else {
    String::new()
  };

  Ok(build_unified_diff(
    &sanitized_relative_path,
    &previous_content,
    next_content,
  ))
}

fn visit_directory(
  workspace_root: &Path,
  current_dir: &Path,
  normalized_query: &str,
  max_results: usize,
  results: &mut Vec<SearchMatch>,
) -> Result<()> {
  if results.len() >= max_results {
    return Ok(());
  }

  let mut entries = fs::read_dir(current_dir)
    .with_context(|| format!("failed to read directory {}", current_dir.display()))?
    .filter_map(|entry| entry.ok())
    .collect::<Vec<_>>();
  entries.sort_by_key(|entry| entry.path());

  for entry in entries {
    if results.len() >= max_results {
      break;
    }

    let path = entry.path();
    let metadata = entry
      .metadata()
      .with_context(|| format!("failed to read metadata for {}", path.display()))?;

    if metadata.is_dir() {
      visit_directory(
        workspace_root,
        &path,
        normalized_query,
        max_results,
        results,
      )?;
      continue;
    }

    if !metadata.is_file() || metadata.len() > 256 * 1024 {
      continue;
    }

    let content = match fs::read(&path) {
      Ok(content) => content,
      Err(_) => continue,
    };
    if content.contains(&0) {
      continue;
    }

    let text = String::from_utf8_lossy(&content);
    for (index, line) in text.lines().enumerate() {
      if !line.to_lowercase().contains(normalized_query) {
        continue;
      }

      results.push(SearchMatch {
        relative_path: relative_path_string(workspace_root, &path)?,
        line_number: index + 1,
        line: line.trim().to_string(),
      });

      if results.len() >= max_results {
        break;
      }
    }
  }

  Ok(())
}

fn resolve_workspace_path(
  workspace_root: &Path,
  relative_path: &str,
  allow_directory: bool,
) -> Result<PathBuf> {
  let workspace_root = fs::canonicalize(workspace_root).with_context(|| {
    format!(
      "failed to resolve workspace root {}",
      workspace_root.display()
    )
  })?;
  let candidate = workspace_root.join(relative_path);
  let resolved = fs::canonicalize(&candidate)
    .with_context(|| format!("failed to resolve workspace path {}", candidate.display()))?;

  if !resolved.starts_with(&workspace_root) {
    bail!("workspace path escapes the selected workspace");
  }

  let metadata = fs::metadata(&resolved)
    .with_context(|| format!("failed to read metadata for {}", resolved.display()))?;

  if metadata.is_dir() && !allow_directory {
    bail!("workspace path points to a directory");
  }

  Ok(resolved)
}

fn relative_path_string(workspace_root: &Path, target: &Path) -> Result<String> {
  let relative = target
    .strip_prefix(workspace_root)
    .with_context(|| format!("failed to relativize {}", target.display()))?;

  if relative.as_os_str().is_empty() {
    return Ok(".".to_string());
  }

  Ok(relative.to_string_lossy().replace('\\', "/"))
}

fn canonical_workspace_root(workspace_root: &Path) -> Result<PathBuf> {
  fs::canonicalize(workspace_root).with_context(|| {
    format!(
      "failed to resolve workspace root {}",
      workspace_root.display()
    )
  })
}

fn sanitize_relative_path(relative_path: &str) -> Result<String> {
  let path = Path::new(relative_path);
  if path.is_absolute() {
    bail!("workspace path must be relative");
  }

  let mut sanitized = PathBuf::new();
  for component in path.components() {
    match component {
      std::path::Component::CurDir => {}
      std::path::Component::Normal(segment) => sanitized.push(segment),
      _ => bail!("workspace path must stay inside the selected workspace"),
    }
  }

  if sanitized.as_os_str().is_empty() {
    bail!("workspace path must not be empty");
  }

  Ok(sanitized.to_string_lossy().replace('\\', "/"))
}

#[cfg(target_family = "windows")]
fn build_shell_command(command: &str) -> Command {
  let mut process = Command::new("powershell");
  process.args(["-NoProfile", "-Command", command]);
  process
}

#[cfg(not(target_family = "windows"))]
fn build_shell_command(command: &str) -> Command {
  let mut process = Command::new("sh");
  process.args(["-lc", command]);
  process
}

fn truncate_output(output: &str, max_output_bytes: usize) -> String {
  let mut collected = String::new();

  for character in output.chars() {
    if collected.len() + character.len_utf8() > max_output_bytes {
      break;
    }
    collected.push(character);
  }

  collected
}

fn build_unified_diff(relative_path: &str, previous_content: &str, next_content: &str) -> String {
  let previous_lines = collect_diff_lines(previous_content);
  let next_lines = collect_diff_lines(next_content);

  let max_line_count = previous_lines.len().max(next_lines.len());
  let mut diff_lines = vec![
    format!("--- a/{relative_path}"),
    format!("+++ b/{relative_path}"),
    "@@".to_string(),
  ];

  if previous_lines == next_lines {
    diff_lines.push("  [no content changes]".to_string());
    return diff_lines.join("\n");
  }

  for index in 0..max_line_count {
    match (previous_lines.get(index), next_lines.get(index)) {
      (Some(previous_line), Some(next_line)) if previous_line == next_line => {
        diff_lines.push(format!(" {}", previous_line));
      }
      (Some(previous_line), Some(next_line)) => {
        diff_lines.push(format!("-{}", previous_line));
        diff_lines.push(format!("+{}", next_line));
      }
      (Some(previous_line), None) => {
        diff_lines.push(format!("-{}", previous_line));
      }
      (None, Some(next_line)) => {
        diff_lines.push(format!("+{}", next_line));
      }
      (None, None) => {}
    }
  }

  diff_lines.join("\n")
}

fn collect_diff_lines(content: &str) -> Vec<String> {
  if content.is_empty() {
    return vec![];
  }

  let mut lines = content.lines().map(ToString::to_string).collect::<Vec<_>>();
  if content.ends_with('\n') {
    lines.push(String::new());
  }
  lines
}
