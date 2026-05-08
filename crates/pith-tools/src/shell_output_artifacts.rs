use std::env;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{bail, Context, Result};

const SHELL_OUTPUT_ARTIFACT_RETAINED_RUNS: usize = 20;

pub(crate) fn shell_output_artifact_directory() -> Result<PathBuf> {
  let root = shell_output_artifact_root();
  ensure_shell_output_artifact_root(&root)?;
  prune_shell_output_artifact_root(&root, SHELL_OUTPUT_ARTIFACT_RETAINED_RUNS);
  create_shell_output_artifact_run_directory(&root)
}

pub(crate) fn discard_shell_output_artifact_directory(artifact_directory: &Path) {
  let Ok(metadata) = fs::symlink_metadata(artifact_directory) else {
    return;
  };

  if metadata.file_type().is_symlink() || !metadata.is_dir() {
    let _ = fs::remove_file(artifact_directory);
    return;
  }

  let _ = fs::remove_dir_all(artifact_directory);
}

fn shell_output_artifact_root() -> PathBuf {
  if let Ok(data_dir) = env::var("PITH_DATA_DIR") {
    return PathBuf::from(data_dir)
      .join("artifacts")
      .join("sandbox-output");
  }

  if let Ok(home_dir) = env::var("HOME") {
    return PathBuf::from(home_dir)
      .join(".pith")
      .join("artifacts")
      .join("sandbox-output");
  }

  if let Ok(home_dir) = env::var("USERPROFILE") {
    return PathBuf::from(home_dir)
      .join(".pith")
      .join("artifacts")
      .join("sandbox-output");
  }

  env::temp_dir()
    .join("Pith")
    .join("artifacts")
    .join("sandbox-output")
}

fn ensure_shell_output_artifact_root(root: &Path) -> Result<()> {
  fs::create_dir_all(root).with_context(|| {
    format!(
      "failed to create shell output artifact root {}",
      root.display()
    )
  })?;
  let metadata = fs::symlink_metadata(root).with_context(|| {
    format!(
      "failed to inspect shell output artifact root {}",
      root.display()
    )
  })?;
  if metadata.file_type().is_symlink() || !metadata.is_dir() {
    bail!(
      "shell output artifact root must be a real directory: {}",
      root.display()
    );
  }
  Ok(())
}

fn create_shell_output_artifact_run_directory(root: &Path) -> Result<PathBuf> {
  for _ in 0..16 {
    let run_directory = root.join(shell_output_artifact_run_name());
    match fs::create_dir(&run_directory) {
      Ok(()) => return Ok(run_directory),
      Err(error) if error.kind() == ErrorKind::AlreadyExists => {}
      Err(error) => {
        return Err(error).with_context(|| {
          format!(
            "failed to create shell output artifact directory {}",
            run_directory.display()
          )
        });
      }
    }
  }

  bail!(
    "failed to allocate a unique shell output artifact directory under {}",
    root.display()
  )
}

#[derive(Debug)]
struct ShellOutputArtifactRun {
  path: PathBuf,
  modified_at: SystemTime,
}

fn prune_shell_output_artifact_root(root: &Path, retained_runs: usize) {
  let Ok(entries) = fs::read_dir(root) else {
    return;
  };
  let mut runs = entries
    .filter_map(|entry| entry.ok())
    .filter_map(|entry| shell_output_artifact_run(entry.path()))
    .collect::<Vec<_>>();
  if runs.len() <= retained_runs {
    return;
  }

  runs.sort_by(|left, right| {
    left
      .modified_at
      .cmp(&right.modified_at)
      .then_with(|| left.path.cmp(&right.path))
  });
  let removable_count = runs.len().saturating_sub(retained_runs);
  for run in runs.into_iter().take(removable_count) {
    let _ = fs::remove_dir_all(run.path);
  }
}

fn shell_output_artifact_run(path: PathBuf) -> Option<ShellOutputArtifactRun> {
  let file_name = path.file_name()?.to_string_lossy();
  if !file_name.starts_with("run-") {
    return None;
  }

  let metadata = fs::symlink_metadata(&path).ok()?;
  if metadata.file_type().is_symlink() || !metadata.is_dir() {
    return None;
  }

  Some(ShellOutputArtifactRun {
    path,
    modified_at: metadata.modified().unwrap_or(UNIX_EPOCH),
  })
}

fn shell_output_artifact_run_name() -> String {
  let nonce = SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .expect("system time")
    .as_nanos();
  format!("run-{nonce}")
}

#[cfg(test)]
mod tests {
  use std::fs;
  use std::path::PathBuf;
  use std::time::{SystemTime, UNIX_EPOCH};

  use super::*;

  #[test]
  fn shell_output_artifact_pruning_keeps_recent_runs() {
    let root = unique_temp_directory("artifact-prune");
    fs::create_dir_all(&root).expect("artifact root");
    for index in 0..5 {
      fs::create_dir_all(root.join(format!("run-{index:02}"))).expect("artifact run");
    }
    fs::create_dir_all(root.join("manual-note")).expect("manual directory");

    prune_shell_output_artifact_root(&root, 2);

    assert!(!root.join("run-00").exists());
    assert!(!root.join("run-01").exists());
    assert!(!root.join("run-02").exists());
    assert!(root.join("run-03").exists());
    assert!(root.join("run-04").exists());
    assert!(root.join("manual-note").exists());

    let _ = fs::remove_dir_all(root);
  }

  #[cfg(unix)]
  #[test]
  fn shell_output_artifact_cleanup_removes_symlink_without_following_it() {
    use std::os::unix::fs::symlink;

    let root = unique_temp_directory("artifact-cleanup-symlink");
    let outside = unique_temp_directory("artifact-cleanup-outside");
    let artifact_directory = root.join("run-linked");
    fs::create_dir_all(&root).expect("artifact root");
    fs::create_dir_all(&outside).expect("outside");
    fs::write(outside.join("keep.txt"), "keep").expect("outside file");
    symlink(&outside, &artifact_directory).expect("run symlink");

    discard_shell_output_artifact_directory(&artifact_directory);

    assert!(!artifact_directory.exists());
    assert!(outside.join("keep.txt").exists());

    let _ = fs::remove_dir_all(root);
    let _ = fs::remove_dir_all(outside);
  }

  #[cfg(unix)]
  #[test]
  fn shell_output_artifact_pruning_ignores_symlinked_runs() {
    use std::os::unix::fs::symlink;

    let root = unique_temp_directory("artifact-prune-symlink");
    let outside = unique_temp_directory("artifact-prune-outside");
    fs::create_dir_all(&root).expect("artifact root");
    fs::create_dir_all(&outside).expect("outside");
    fs::write(outside.join("keep.txt"), "keep").expect("outside file");
    symlink(&outside, root.join("run-linked")).expect("run symlink");
    for index in 0..3 {
      fs::create_dir_all(root.join(format!("run-{index:02}"))).expect("artifact run");
    }

    prune_shell_output_artifact_root(&root, 1);

    assert!(root.join("run-linked").exists());
    assert!(outside.join("keep.txt").exists());

    let _ = fs::remove_dir_all(root);
    let _ = fs::remove_dir_all(outside);
  }

  #[test]
  fn shell_output_artifact_directory_creates_real_run_directory() {
    let root = unique_temp_directory("artifact-run-directory");

    ensure_shell_output_artifact_root(&root).expect("artifact root");
    let run_directory =
      create_shell_output_artifact_run_directory(&root).expect("artifact run directory");

    assert!(run_directory.is_dir());
    assert!(run_directory.starts_with(&root));

    let _ = fs::remove_dir_all(root);
  }

  #[cfg(unix)]
  #[test]
  fn shell_output_artifact_root_rejects_symlink() {
    use std::os::unix::fs::symlink;

    let root_parent = unique_temp_directory("artifact-root-parent");
    let outside = unique_temp_directory("artifact-root-outside");
    let root = root_parent.join("sandbox-output");
    fs::create_dir_all(&root_parent).expect("root parent");
    fs::create_dir_all(&outside).expect("outside");
    symlink(&outside, &root).expect("artifact root symlink");

    let error = ensure_shell_output_artifact_root(&root).expect_err("symlink root should fail");

    assert!(error.to_string().contains("must be a real directory"));

    let _ = fs::remove_dir_all(root_parent);
    let _ = fs::remove_dir_all(outside);
  }

  fn unique_temp_directory(prefix: &str) -> PathBuf {
    let nonce = SystemTime::now()
      .duration_since(UNIX_EPOCH)
      .expect("clock")
      .as_nanos();
    std::env::temp_dir().join(format!("pith-tools-{prefix}-{nonce}"))
  }
}
