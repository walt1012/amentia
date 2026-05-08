use std::path::Path;

use anyhow::{bail, Context, Result};

use crate::bounded_file::{read_text_prefix, text_prefix};
use crate::paths::{
  canonical_workspace_root, sanitize_relative_path, validate_workspace_write_target,
};

const DIFF_PREVIEW_MAX_BYTES: usize = 128 * 1024;

pub fn generate_diff(
  workspace_root: &Path,
  relative_path: &str,
  next_content: &str,
) -> Result<String> {
  let workspace_root = canonical_workspace_root(workspace_root)?;
  let sanitized_relative_path = sanitize_relative_path(relative_path)?;
  let target = workspace_root.join(&sanitized_relative_path);

  validate_workspace_write_target(&workspace_root, &target)?;
  if target.is_dir() {
    bail!("workspace path points to a directory");
  }

  let previous_preview = if target.is_file() {
    read_text_prefix(&target, DIFF_PREVIEW_MAX_BYTES)
  } else {
    Ok(text_prefix("", DIFF_PREVIEW_MAX_BYTES))
  };
  let previous_preview = previous_preview
    .with_context(|| format!("failed to read file {}", target.display()))?;
  let next_preview = text_prefix(next_content, DIFF_PREVIEW_MAX_BYTES);
  let mut diff = build_unified_diff(
    &sanitized_relative_path,
    &previous_preview.content,
    &next_preview.content,
  );
  if previous_preview.is_truncated || next_preview.is_truncated {
    diff = format!(
      "[diff preview truncated to {} bytes per side]\n{}",
      DIFF_PREVIEW_MAX_BYTES, diff
    );
  }

  Ok(diff)
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

#[cfg(test)]
mod tests {
  use std::fs;
  use std::path::PathBuf;
  use std::time::{SystemTime, UNIX_EPOCH};

  use super::*;

  #[cfg(unix)]
  #[test]
  fn generate_diff_rejects_symlink_escape() {
    use std::os::unix::fs::symlink;

    let workspace = unique_temp_workspace("diff-symlink");
    let outside = unique_temp_workspace("diff-outside");
    fs::create_dir_all(&workspace).expect("workspace");
    fs::create_dir_all(&outside).expect("outside");
    fs::write(outside.join("target.txt"), "outside").expect("outside file");
    symlink(
      outside.join("target.txt"),
      workspace.join("linked-target.txt"),
    )
    .expect("symlink");

    let error = generate_diff(&workspace, "linked-target.txt", "inside")
      .expect_err("symlink diff should fail");

    assert!(error
      .to_string()
      .contains("workspace path points to a symlink"));

    let _ = fs::remove_dir_all(workspace);
    let _ = fs::remove_dir_all(outside);
  }

  #[cfg(unix)]
  #[test]
  fn generate_diff_rejects_parent_symlink() {
    use std::os::unix::fs::symlink;

    let workspace = unique_temp_workspace("diff-parent-symlink");
    fs::create_dir_all(workspace.join("real")).expect("workspace real dir");
    fs::write(workspace.join("real/target.txt"), "inside").expect("inside file");
    symlink(workspace.join("real"), workspace.join("alias")).expect("symlink");

    let error = generate_diff(&workspace, "alias/target.txt", "next")
      .expect_err("parent symlink diff should fail");

    assert!(error
      .to_string()
      .contains("workspace path crosses a symlink"));

    let _ = fs::remove_dir_all(workspace);
  }

  #[test]
  fn generate_diff_bounds_large_file_preview() {
    let workspace = unique_temp_workspace("diff-bounded");
    fs::create_dir_all(&workspace).expect("workspace");
    fs::write(
      workspace.join("large.txt"),
      format!("{}\nold tail\n", "old\n".repeat(70_000)),
    )
    .expect("large file");

    let diff = generate_diff(&workspace, "large.txt", "new content").expect("diff");

    assert!(diff.contains("diff preview truncated"));
    assert!(diff.len() < 300_000);

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
