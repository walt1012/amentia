use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::types::ShellSandboxSummary;

const SHELL_SANDBOX_TEMP_DIR: &str = ".pith/sandbox-tmp";

pub(crate) struct ShellSandboxPlan {
  pub(crate) policy: pith_sandbox::SandboxPolicy,
  pub(crate) summary: ShellSandboxSummary,
}

pub(crate) fn shell_sandbox_plan(workspace_root: &Path) -> ShellSandboxPlan {
  let policy = shell_sandbox_policy(workspace_root);
  let status = pith_sandbox::native_sandbox_status(&policy);

  ShellSandboxPlan {
    policy,
    summary: shell_sandbox_summary_from_status(status),
  }
}

pub(crate) fn shell_sandbox_summary(workspace_root: &Path) -> ShellSandboxSummary {
  let status = shell_sandbox_status(workspace_root);

  shell_sandbox_summary_from_status(status)
}

fn shell_sandbox_summary_from_status(
  status: pith_sandbox::NativeSandboxStatus,
) -> ShellSandboxSummary {
  ShellSandboxSummary {
    mode: status.mode,
    backend: status.backend,
    available: status.available,
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
  _sandbox: &ShellSandboxSummary,
) -> Result<()> {
  let temporary_root = shell_sandbox_temp_root(workspace_root);
  pith_sandbox::prepare_workspace_temporary_root(workspace_root, &temporary_root)
    .map_err(anyhow::Error::msg)
}

#[cfg(test)]
mod tests {
  use std::fs;
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
      available: true,
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

  #[test]
  fn prepare_environment_creates_inactive_sandbox_temporary_root() {
    let workspace = unique_temp_workspace("shell-sandbox-inactive-temp");
    let temporary_root = shell_sandbox_temp_root(&workspace);
    fs::create_dir_all(&workspace).expect("workspace");
    let sandbox = ShellSandboxSummary {
      mode: "workspaceReadWrite".to_string(),
      backend: "processOnly".to_string(),
      available: false,
      active: false,
      network_allowed: false,
      temporary_root: Some(temporary_root.display().to_string()),
      writable_roots: vec![workspace.display().to_string()],
      detail: "test sandbox".to_string(),
    };

    prepare_shell_sandbox_environment(&workspace, &sandbox).expect("prepare sandbox");

    assert!(temporary_root.is_dir());

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
      available: true,
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

  #[cfg(unix)]
  #[test]
  fn prepare_environment_rejects_parent_symlink_escape() {
    use std::os::unix::fs::symlink;

    let workspace = unique_temp_workspace("shell-sandbox-parent-symlink");
    let outside = unique_temp_workspace("shell-sandbox-parent-outside");
    let temporary_root = shell_sandbox_temp_root(&workspace);
    let parent = temporary_root.parent().expect("temporary parent");
    fs::create_dir_all(parent.parent().expect("pith directory")).expect("pith directory");
    fs::create_dir_all(&outside).expect("outside");
    symlink(&outside, parent).expect("temporary parent symlink");
    let sandbox = ShellSandboxSummary {
      mode: "workspaceReadWrite".to_string(),
      backend: "macosSeatbelt".to_string(),
      available: true,
      active: true,
      network_allowed: false,
      temporary_root: Some(temporary_root.display().to_string()),
      writable_roots: vec![
        workspace.display().to_string(),
        temporary_root.display().to_string(),
      ],
      detail: "test sandbox".to_string(),
    };

    let error = prepare_shell_sandbox_environment(&workspace, &sandbox)
      .expect_err("parent symlink should fail");

    assert!(error.to_string().contains("crosses a symlink"));
    assert!(!outside.join("sandbox-tmp").exists());

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
