use std::fs;
use std::path::Path;

use anyhow::{bail, Context, Result};

use crate::bounded_file::read_text_prefix;
use crate::paths::{
  canonical_workspace_root, relative_path_string, resolve_workspace_path, sanitize_relative_path,
  validate_workspace_write_parent, validate_workspace_write_target,
};
use crate::types::{DirectoryEntry, ReadFileResult};

const LIST_DIRECTORY_MAX_SCANNED_ENTRIES: usize = 5_000;

pub fn list_directory_max_scanned_entries() -> usize {
  LIST_DIRECTORY_MAX_SCANNED_ENTRIES
}

pub fn list_directory(
  workspace_root: &Path,
  relative_path: Option<&str>,
  limit: usize,
) -> Result<Vec<DirectoryEntry>> {
  list_directory_with_cancellation(workspace_root, relative_path, limit, || false)
}

pub fn list_directory_with_cancellation<F>(
  workspace_root: &Path,
  relative_path: Option<&str>,
  limit: usize,
  is_cancelled: F,
) -> Result<Vec<DirectoryEntry>>
where
  F: Fn() -> bool,
{
  list_directory_with_entry_limit(
    workspace_root,
    relative_path,
    limit,
    LIST_DIRECTORY_MAX_SCANNED_ENTRIES,
    is_cancelled,
  )
}

fn list_directory_with_entry_limit<F>(
  workspace_root: &Path,
  relative_path: Option<&str>,
  limit: usize,
  max_scanned_entries: usize,
  is_cancelled: F,
) -> Result<Vec<DirectoryEntry>>
where
  F: Fn() -> bool,
{
  if is_cancelled() {
    bail!("directory listing cancelled");
  }
  let target = resolve_workspace_path(workspace_root, relative_path.unwrap_or("."), true)?;
  let workspace_root = canonical_workspace_root(workspace_root)?;

  let mut entries = Vec::new();
  for (scanned_entries, entry) in fs::read_dir(&target)
    .with_context(|| format!("failed to read directory {}", target.display()))?
    .filter_map(|entry| entry.ok())
    .enumerate()
  {
    if is_cancelled() {
      bail!("directory listing cancelled");
    }
    if scanned_entries >= max_scanned_entries {
      bail!("directory listing scanned too many entries; open a narrower folder");
    }

    let path = entry.path();
    let metadata = fs::symlink_metadata(&path)
      .with_context(|| format!("failed to read metadata for {}", path.display()))?;
    let entry_type = if metadata.file_type().is_symlink() {
      "symlink"
    } else if metadata.is_dir() {
      "directory"
    } else {
      "file"
    };
    let relative_path = relative_path_string(&workspace_root, &path)?;

    entries.push(DirectoryEntry {
      name: entry.file_name().to_string_lossy().into_owned(),
      relative_path,
      entry_type: entry_type.to_string(),
    });
  }

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
  let workspace_root = canonical_workspace_root(workspace_root)?;
  let preview = read_text_prefix(&target, max_bytes)?;

  Ok(ReadFileResult {
    relative_path: relative_path_string(&workspace_root, &target)?,
    content: preview.content,
    is_truncated: preview.is_truncated,
  })
}

pub fn write_file(workspace_root: &Path, relative_path: &str, content: &str) -> Result<String> {
  let workspace_root = canonical_workspace_root(workspace_root)?;
  let sanitized_relative_path = sanitize_relative_path(relative_path)?;
  let target = workspace_root.join(&sanitized_relative_path);

  if let Some(parent) = target.parent() {
    validate_workspace_write_parent(&workspace_root, parent)?;
    fs::create_dir_all(parent)
      .with_context(|| format!("failed to create directory {}", parent.display()))?;
    validate_workspace_write_parent(&workspace_root, parent)?;
  }

  validate_workspace_write_target(&workspace_root, &target)?;
  if target.is_dir() {
    bail!("workspace path points to a directory");
  }

  fs::write(&target, content)
    .with_context(|| format!("failed to write file {}", target.display()))?;

  Ok(sanitized_relative_path)
}

#[cfg(test)]
mod tests {
  use std::fs;
  use std::path::PathBuf;
  use std::time::{SystemTime, UNIX_EPOCH};

  use super::*;

  #[cfg(unix)]
  #[test]
  fn write_file_rejects_symlink_escape() {
    use std::os::unix::fs::symlink;

    let workspace = unique_temp_workspace("write-symlink");
    let outside = unique_temp_workspace("write-outside");
    fs::create_dir_all(&workspace).expect("workspace");
    fs::create_dir_all(&outside).expect("outside");
    symlink(&outside, workspace.join("outside-link")).expect("symlink");

    let error = write_file(&workspace, "outside-link/owned.txt", "nope")
      .expect_err("symlink escape should fail");

    assert!(error.to_string().contains("workspace path escapes"));
    assert!(!outside.join("owned.txt").exists());

    let _ = fs::remove_dir_all(workspace);
    let _ = fs::remove_dir_all(outside);
  }

  #[cfg(unix)]
  #[test]
  fn write_file_rejects_target_symlink() {
    use std::os::unix::fs::symlink;

    let workspace = unique_temp_workspace("write-target-symlink");
    let outside = unique_temp_workspace("write-target-outside");
    fs::create_dir_all(&workspace).expect("workspace");
    fs::create_dir_all(&outside).expect("outside");
    fs::write(outside.join("target.txt"), "outside").expect("outside file");
    symlink(
      outside.join("target.txt"),
      workspace.join("linked-target.txt"),
    )
    .expect("symlink");

    let error = write_file(&workspace, "linked-target.txt", "inside")
      .expect_err("target symlink should fail");

    assert!(error
      .to_string()
      .contains("workspace path points to a symlink"));
    assert_eq!(
      fs::read_to_string(outside.join("target.txt")).expect("outside file"),
      "outside"
    );

    let _ = fs::remove_dir_all(workspace);
    let _ = fs::remove_dir_all(outside);
  }

  #[cfg(unix)]
  #[test]
  fn write_file_rejects_parent_symlink() {
    use std::os::unix::fs::symlink;

    let workspace = unique_temp_workspace("write-parent-symlink");
    fs::create_dir_all(workspace.join("real")).expect("workspace real dir");
    symlink(workspace.join("real"), workspace.join("alias")).expect("symlink");

    let error = write_file(&workspace, "alias/created.txt", "inside")
      .expect_err("parent symlink should fail");

    assert!(error
      .to_string()
      .contains("workspace path crosses a symlink"));
    assert!(!workspace.join("real/created.txt").exists());

    let _ = fs::remove_dir_all(workspace);
  }

  #[cfg(unix)]
  #[test]
  fn list_directory_reports_symlinks_without_following() {
    use std::os::unix::fs::symlink;

    let workspace = unique_temp_workspace("list-symlink");
    let outside = unique_temp_workspace("list-outside");
    fs::create_dir_all(&workspace).expect("workspace");
    fs::create_dir_all(&outside).expect("outside");
    fs::write(outside.join("secret.txt"), "hidden").expect("outside file");
    symlink(&outside, workspace.join("outside-link")).expect("symlink");

    let entries = list_directory(&workspace, None, 10).expect("directory entries");

    let symlink_entry = entries
      .iter()
      .find(|entry| entry.relative_path == "outside-link")
      .expect("symlink entry");
    assert_eq!(symlink_entry.entry_type, "symlink");

    let _ = fs::remove_dir_all(workspace);
    let _ = fs::remove_dir_all(outside);
  }

  #[test]
  fn list_directory_stops_when_cancelled() {
    let workspace = unique_temp_workspace("list-cancel");
    fs::create_dir_all(&workspace).expect("workspace");
    fs::write(workspace.join("inside.txt"), "visible").expect("inside file");

    let error =
      list_directory_with_cancellation(&workspace, None, 10, || true).expect_err("cancelled");

    assert!(error.to_string().contains("directory listing cancelled"));

    let _ = fs::remove_dir_all(workspace);
  }

  #[test]
  fn list_directory_stops_at_entry_budget() {
    let workspace = unique_temp_workspace("list-budget");
    fs::create_dir_all(&workspace).expect("workspace");
    fs::write(workspace.join("one.txt"), "one").expect("one file");
    fs::write(workspace.join("two.txt"), "two").expect("two file");
    fs::write(workspace.join("three.txt"), "three").expect("three file");

    let error =
      list_directory_with_entry_limit(&workspace, None, 10, 2, || false).expect_err("entry budget");

    assert!(error
      .to_string()
      .contains("directory listing scanned too many entries"));

    let _ = fs::remove_dir_all(workspace);
  }

  #[test]
  fn read_file_uses_bounded_preview() {
    let workspace = unique_temp_workspace("read-bounded");
    fs::create_dir_all(&workspace).expect("workspace");
    fs::write(workspace.join("large.txt"), "x".repeat(4096)).expect("large file");

    let result = read_file(&workspace, "large.txt", 128).expect("read result");

    assert_eq!(result.content.len(), 128);
    assert!(result.is_truncated);

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
