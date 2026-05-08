use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::types::ShellSandboxSummary;

const SHELL_SANDBOX_TEMP_DIR: &str = ".pith/sandbox-tmp";

pub(crate) fn shell_sandbox_summary(workspace_root: &Path) -> ShellSandboxSummary {
  let status = shell_sandbox_status(workspace_root);

  ShellSandboxSummary {
    mode: status.mode,
    backend: status.backend,
    active: status.active,
    network_allowed: status.network_allowed,
    temporary_root: status.temporary_root,
    writable_roots: status.writable_roots,
    detail: status.detail,
  }
}

pub(crate) fn shell_sandbox_status(workspace_root: &Path) -> pith_sandbox::NativeSandboxStatus {
  let policy = shell_sandbox_policy(workspace_root);
  pith_sandbox::native_sandbox_status(&policy)
}

pub(crate) fn shell_sandbox_policy(workspace_root: &Path) -> pith_sandbox::SandboxPolicy {
  pith_sandbox::SandboxPolicy::workspace_read_write(workspace_root)
    .with_temporary_root(shell_sandbox_temp_root(workspace_root))
}

pub(crate) fn shell_sandbox_temp_root(workspace_root: &Path) -> PathBuf {
  workspace_root.join(SHELL_SANDBOX_TEMP_DIR)
}

pub(crate) fn prepare_shell_sandbox_environment(
  workspace_root: &Path,
  sandbox: &ShellSandboxSummary,
) -> Result<()> {
  if sandbox.active {
    let temporary_root = shell_sandbox_temp_root(workspace_root);
    clear_sandbox_temporary_root(&temporary_root)?;
    fs::create_dir_all(&temporary_root).with_context(|| {
      format!(
        "failed to create sandbox temporary directory {}",
        temporary_root.display()
      )
    })?;
  }

  Ok(())
}

fn clear_sandbox_temporary_root(temporary_root: &Path) -> Result<()> {
  let metadata = match fs::symlink_metadata(temporary_root) {
    Ok(metadata) => metadata,
    Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
    Err(error) => {
      return Err(error).with_context(|| {
        format!(
          "failed to inspect sandbox temporary directory {}",
          temporary_root.display()
        )
      });
    }
  };

  if metadata.file_type().is_symlink() || metadata.is_file() {
    fs::remove_file(temporary_root).with_context(|| {
      format!(
        "failed to clear sandbox temporary path {}",
        temporary_root.display()
      )
    })?;
  } else if metadata.is_dir() {
    fs::remove_dir_all(temporary_root).with_context(|| {
      format!(
        "failed to clear sandbox temporary directory {}",
        temporary_root.display()
      )
    })?;
  } else {
    fs::remove_file(temporary_root).with_context(|| {
      format!(
        "failed to clear sandbox temporary path {}",
        temporary_root.display()
      )
    })?;
  }

  Ok(())
}

#[cfg(test)]
mod tests {
  use std::time::{SystemTime, UNIX_EPOCH};

  use super::*;

  #[test]
  fn prepare_environment_clears_active_sandbox_temporary_root() {
    let workspace = unique_temp_workspace("shell-sandbox-cleanup");
    let temporary_root = shell_sandbox_temp_root(&workspace);
    let stale_file = temporary_root.join("stale.txt");
    fs::create_dir_all(&temporary_root).expect("temporary root");
    fs::write(&stale_file, "stale").expect("stale file");
    let sandbox = ShellSandboxSummary {
      mode: "workspaceReadWrite".to_string(),
      backend: "macosSeatbelt".to_string(),
      active: true,
      network_allowed: false,
      temporary_root: Some(temporary_root.display().to_string()),
      writable_roots: vec![
        workspace.display().to_string(),
        temporary_root.display().to_string(),
      ],
      detail: "test sandbox".to_string(),
    };

    prepare_shell_sandbox_environment(&workspace, &sandbox).expect("prepare sandbox");

    assert!(temporary_root.is_dir());
    assert!(!stale_file.exists());

    let _ = fs::remove_dir_all(workspace);
  }

  #[cfg(unix)]
  #[test]
  fn prepare_environment_removes_temporary_root_symlink_without_following_it() {
    use std::os::unix::fs::symlink;

    let workspace = unique_temp_workspace("shell-sandbox-symlink-cleanup");
    let outside = unique_temp_workspace("shell-sandbox-outside");
    let temporary_root = shell_sandbox_temp_root(&workspace);
    let outside_file = outside.join("keep.txt");
    fs::create_dir_all(temporary_root.parent().expect("temporary parent")).expect("parent");
    fs::create_dir_all(&outside).expect("outside");
    fs::write(&outside_file, "keep").expect("outside file");
    symlink(&outside, &temporary_root).expect("temporary root symlink");
    let sandbox = ShellSandboxSummary {
      mode: "workspaceReadWrite".to_string(),
      backend: "macosSeatbelt".to_string(),
      active: true,
      network_allowed: false,
      temporary_root: Some(temporary_root.display().to_string()),
      writable_roots: vec![
        workspace.display().to_string(),
        temporary_root.display().to_string(),
      ],
      detail: "test sandbox".to_string(),
    };

    prepare_shell_sandbox_environment(&workspace, &sandbox).expect("prepare sandbox");

    assert!(temporary_root.is_dir());
    assert!(outside_file.is_file());

    let _ = fs::remove_dir_all(workspace);
    let _ = fs::remove_dir_all(outside);
  }

  fn unique_temp_workspace(prefix: &str) -> PathBuf {
    let nonce = SystemTime::now()
      .duration_since(UNIX_EPOCH)
      .expect("clock")
      .as_nanos();
    std::env::temp_dir().join(format!("pith-tools-{prefix}-{nonce}"))
  }
}
