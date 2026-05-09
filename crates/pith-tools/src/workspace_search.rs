use std::fs;
use std::path::Path;

use anyhow::{bail, Context, Result};

use crate::paths::{canonical_workspace_root, relative_path_string};
use crate::types::SearchMatch;

pub fn search_files(
  workspace_root: &Path,
  query: &str,
  max_results: usize,
) -> Result<Vec<SearchMatch>> {
  search_files_with_cancellation(workspace_root, query, max_results, || false)
}

pub fn search_files_with_cancellation<F>(
  workspace_root: &Path,
  query: &str,
  max_results: usize,
  is_cancelled: F,
) -> Result<Vec<SearchMatch>>
where
  F: Fn() -> bool,
{
  let workspace_root = canonical_workspace_root(workspace_root)?;
  let normalized_query = query.trim().to_lowercase();

  if normalized_query.is_empty() {
    bail!("search query must not be empty");
  }
  if is_cancelled() {
    bail!("search cancelled");
  }

  let mut results = vec![];
  visit_directory(
    &workspace_root,
    &workspace_root,
    &normalized_query,
    max_results,
    &is_cancelled,
    &mut results,
  )?;

  Ok(results)
}

fn visit_directory<F>(
  workspace_root: &Path,
  current_dir: &Path,
  normalized_query: &str,
  max_results: usize,
  is_cancelled: &F,
  results: &mut Vec<SearchMatch>,
) -> Result<()>
where
  F: Fn() -> bool,
{
  if is_cancelled() {
    bail!("search cancelled");
  }
  if results.len() >= max_results {
    return Ok(());
  }

  let mut entries = fs::read_dir(current_dir)
    .with_context(|| format!("failed to read directory {}", current_dir.display()))?
    .filter_map(|entry| entry.ok())
    .collect::<Vec<_>>();
  entries.sort_by_key(|entry| entry.path());

  for entry in entries {
    if is_cancelled() {
      bail!("search cancelled");
    }
    if results.len() >= max_results {
      break;
    }

    let path = entry.path();
    let metadata = fs::symlink_metadata(&path)
      .with_context(|| format!("failed to read metadata for {}", path.display()))?;
    if metadata.file_type().is_symlink() {
      continue;
    }

    if metadata.is_dir() {
      let resolved_directory = fs::canonicalize(&path)
        .with_context(|| format!("failed to resolve directory {}", path.display()))?;
      if !resolved_directory.starts_with(workspace_root) {
        continue;
      }
      visit_directory(
        workspace_root,
        &resolved_directory,
        normalized_query,
        max_results,
        is_cancelled,
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
      if is_cancelled() {
        bail!("search cancelled");
      }
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

#[cfg(test)]
mod tests {
  use std::fs;
  use std::path::PathBuf;
  use std::time::{SystemTime, UNIX_EPOCH};

  use super::*;

  #[cfg(unix)]
  #[test]
  fn search_files_skips_symlinked_directories() {
    use std::os::unix::fs::symlink;

    let workspace = unique_temp_workspace("search-symlink");
    let outside = unique_temp_workspace("search-outside");
    fs::create_dir_all(&workspace).expect("workspace");
    fs::create_dir_all(&outside).expect("outside");
    fs::write(workspace.join("inside.txt"), "visible needle").expect("inside file");
    fs::write(outside.join("secret.txt"), "hidden needle").expect("outside file");
    symlink(&outside, workspace.join("outside-link")).expect("symlink");

    let matches = search_files(&workspace, "needle", 10).expect("search");

    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].relative_path, "inside.txt");

    let _ = fs::remove_dir_all(workspace);
    let _ = fs::remove_dir_all(outside);
  }

  #[test]
  fn search_files_stops_when_cancelled() {
    let workspace = unique_temp_workspace("search-cancel");
    fs::create_dir_all(&workspace).expect("workspace");
    fs::write(workspace.join("inside.txt"), "visible needle").expect("inside file");

    let error =
      search_files_with_cancellation(&workspace, "needle", 10, || true).expect_err("cancelled");

    assert!(error.to_string().contains("search cancelled"));

    let _ = fs::remove_dir_all(workspace);
  }

  fn unique_temp_workspace(prefix: &str) -> PathBuf {
    let nonce = SystemTime::now()
      .duration_since(UNIX_EPOCH)
      .expect("clock")
      .as_nanos();
    std::env::temp_dir().join(format!("pith-tools-{prefix}-{nonce}"))
  }
}
