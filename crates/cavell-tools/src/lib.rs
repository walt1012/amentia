use std::fs;
use std::path::{Path, PathBuf};

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
