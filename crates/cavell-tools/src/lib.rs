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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchMatch {
  pub relative_path: String,
  pub line_number: usize,
  pub line: String,
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
    format!("failed to resolve workspace root {}", workspace_root.display())
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
  fs::canonicalize(workspace_root)
    .with_context(|| format!("failed to resolve workspace root {}", workspace_root.display()))
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
