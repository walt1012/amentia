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
    temporary_root: status.temporary_root,
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
    fs::create_dir_all(&temporary_root).with_context(|| {
      format!(
        "failed to create sandbox temporary directory {}",
        temporary_root.display()
      )
    })?;
  }

  Ok(())
}
