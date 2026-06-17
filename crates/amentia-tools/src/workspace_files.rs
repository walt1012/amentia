use std::fs;
use std::path::Path;

use anyhow::{bail, Context, Result};

use crate::bounded_file::read_text_prefix_with_cancellation;
use crate::paths::{
  canonical_workspace_root, relative_path_string, resolve_workspace_path, sanitize_relative_path,
  validate_workspace_write_parent, validate_workspace_write_target,
};
use crate::types::{DirectoryEntry, ReadFileResult, RevertFileChangeResult, WriteFileResult};

const LIST_DIRECTORY_MAX_SCANNED_ENTRIES: usize = 5_000;
const WRITE_FILE_MAX_BYTES: usize = 1024 * 1024;

pub fn list_directory_max_scanned_entries() -> usize {
  LIST_DIRECTORY_MAX_SCANNED_ENTRIES
}

pub fn write_file_max_bytes() -> usize {
  WRITE_FILE_MAX_BYTES
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
  read_file_with_cancellation(workspace_root, relative_path, max_bytes, || false)
}

pub fn read_file_with_cancellation<F>(
  workspace_root: &Path,
  relative_path: &str,
  max_bytes: usize,
  is_cancelled: F,
) -> Result<ReadFileResult>
where
  F: Fn() -> bool,
{
  if is_cancelled() {
    bail!("file read cancelled");
  }
  let target = resolve_workspace_path(workspace_root, relative_path, false)?;
  let workspace_root = canonical_workspace_root(workspace_root)?;
  let metadata = fs::metadata(&target)
    .with_context(|| format!("failed to read metadata for {}", target.display()))?;
  if !metadata.is_file() {
    bail!("workspace path is not a regular file");
  }
  let preview = read_text_prefix_with_cancellation(&target, max_bytes, &is_cancelled)?;

  Ok(ReadFileResult {
    relative_path: relative_path_string(&workspace_root, &target)?,
    content: preview.content,
    is_truncated: preview.is_truncated,
  })
}

pub fn write_file(
  workspace_root: &Path,
  relative_path: &str,
  content: &str,
) -> Result<WriteFileResult> {
  if content.len() > WRITE_FILE_MAX_BYTES {
    bail!("workspace write content exceeds the maximum allowed size");
  }

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

  let previous_content = if target.exists() {
    Some(fs::read(&target).with_context(|| {
      format!(
        "failed to read previous file content for {}",
        target.display()
      )
    })?)
  } else {
    None
  };
  let next_content = content.as_bytes().to_vec();

  fs::write(&target, content)
    .with_context(|| format!("failed to write file {}", target.display()))?;

  Ok(WriteFileResult {
    relative_path: sanitized_relative_path,
    bytes_written: content.len(),
    previous_content,
    next_content,
  })
}

pub fn validate_file_change_revert(
  workspace_root: &Path,
  relative_path: &str,
  expected_current_content: &[u8],
) -> Result<String> {
  let target = resolve_revert_target(workspace_root, relative_path)?;
  let current_content = fs::read(&target).with_context(|| {
    format!(
      "failed to read current file content for {}",
      target.display()
    )
  })?;
  if current_content != expected_current_content {
    bail!("workspace file changed after Amentia wrote it; review it before reverting");
  }

  sanitize_relative_path(relative_path)
}

pub fn revert_file_change(
  workspace_root: &Path,
  relative_path: &str,
  expected_current_content: &[u8],
  previous_content: Option<&[u8]>,
) -> Result<RevertFileChangeResult> {
  let sanitized_relative_path =
    validate_file_change_revert(workspace_root, relative_path, expected_current_content)?;
  let workspace_root = canonical_workspace_root(workspace_root)?;
  let target = workspace_root.join(&sanitized_relative_path);

  match previous_content {
    Some(previous_content) => {
      fs::write(&target, previous_content)
        .with_context(|| format!("failed to restore file {}", target.display()))?;
      Ok(RevertFileChangeResult {
        relative_path: sanitized_relative_path,
        restored_bytes: previous_content.len(),
        deleted_file: false,
      })
    }
    None => {
      fs::remove_file(&target)
        .with_context(|| format!("failed to remove file {}", target.display()))?;
      Ok(RevertFileChangeResult {
        relative_path: sanitized_relative_path,
        restored_bytes: 0,
        deleted_file: true,
      })
    }
  }
}

fn resolve_revert_target(workspace_root: &Path, relative_path: &str) -> Result<std::path::PathBuf> {
  let workspace_root = canonical_workspace_root(workspace_root)?;
  let sanitized_relative_path = sanitize_relative_path(relative_path)?;
  let target = workspace_root.join(&sanitized_relative_path);
  validate_workspace_write_target(&workspace_root, &target)?;
  if target.is_dir() {
    bail!("workspace path points to a directory");
  }

  Ok(target)
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

  #[test]
  fn read_file_stops_when_cancelled() {
    let workspace = unique_temp_workspace("read-cancel");
    fs::create_dir_all(&workspace).expect("workspace");
    fs::write(workspace.join("inside.txt"), "visible").expect("inside file");

    let error =
      read_file_with_cancellation(&workspace, "inside.txt", 128, || true).expect_err("cancelled");

    assert!(error.to_string().contains("file read cancelled"));

    let _ = fs::remove_dir_all(workspace);
  }

  #[test]
  fn write_file_rejects_large_payload() {
    let workspace = unique_temp_workspace("write-large");
    fs::create_dir_all(&workspace).expect("workspace");
    let content = "x".repeat(write_file_max_bytes() + 1);

    let error = write_file(&workspace, "large.txt", &content).expect_err("large payload");

    assert!(error
      .to_string()
      .contains("workspace write content exceeds"));
    assert!(!workspace.join("large.txt").exists());

    let _ = fs::remove_dir_all(workspace);
  }

  #[test]
  fn revert_file_change_removes_new_file_when_content_matches() {
    let workspace = unique_temp_workspace("revert-new-file");
    fs::create_dir_all(&workspace).expect("workspace");
    let write_result = write_file(&workspace, "notes.txt", "created").expect("write file");

    let revert_result = revert_file_change(
      &workspace,
      &write_result.relative_path,
      &write_result.next_content,
      write_result.previous_content.as_deref(),
    )
    .expect("revert file");

    assert_eq!(revert_result.relative_path, "notes.txt");
    assert_eq!(revert_result.restored_bytes, 0);
    assert!(revert_result.deleted_file);
    assert!(!workspace.join("notes.txt").exists());

    let _ = fs::remove_dir_all(workspace);
  }

  #[test]
  fn revert_file_change_rejects_user_modified_file() {
    let workspace = unique_temp_workspace("revert-user-modified");
    fs::create_dir_all(&workspace).expect("workspace");
    let write_result = write_file(&workspace, "notes.txt", "created").expect("write file");
    fs::write(workspace.join("notes.txt"), "user edit").expect("user edit");

    let error = revert_file_change(
      &workspace,
      &write_result.relative_path,
      &write_result.next_content,
      write_result.previous_content.as_deref(),
    )
    .expect_err("modified file should not revert");

    assert!(error
      .to_string()
      .contains("workspace file changed after Amentia wrote it"));
    assert_eq!(
      fs::read_to_string(workspace.join("notes.txt")).expect("read file"),
      "user edit"
    );

    let _ = fs::remove_dir_all(workspace);
  }

  fn unique_temp_workspace(prefix: &str) -> PathBuf {
    let nonce = SystemTime::now()
      .duration_since(UNIX_EPOCH)
      .expect("clock")
      .as_nanos();
    std::env::temp_dir().join(format!("amentia-tools-{prefix}-{nonce}"))
  }
}
