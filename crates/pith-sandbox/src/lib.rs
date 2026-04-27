use std::path::{Path, PathBuf};

const MACOS_SANDBOX_EXEC_PATH: &str = "/usr/bin/sandbox-exec";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SandboxPolicy {
  workspace_root: PathBuf,
  temporary_root: Option<PathBuf>,
  allow_network: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeSandboxStatus {
  pub mode: String,
  pub backend: String,
  pub available: bool,
  pub active: bool,
  pub temporary_root: Option<String>,
  pub detail: String,
}

impl SandboxPolicy {
  pub fn workspace_read_write(workspace_root: impl Into<PathBuf>) -> Self {
    Self {
      workspace_root: workspace_root.into(),
      temporary_root: None,
      allow_network: false,
    }
  }

  pub fn with_temporary_root(mut self, temporary_root: impl Into<PathBuf>) -> Self {
    self.temporary_root = Some(temporary_root.into());
    self
  }

  pub fn workspace_root(&self) -> &Path {
    &self.workspace_root
  }

  pub fn temporary_root(&self) -> Option<&Path> {
    self.temporary_root.as_deref()
  }

  pub fn allow_network(&self) -> bool {
    self.allow_network
  }
}

pub fn workspace_required_status() -> NativeSandboxStatus {
  NativeSandboxStatus {
    mode: "workspaceReadWrite".to_string(),
    backend: native_backend_name().to_string(),
    available: native_sandbox_available(),
    active: false,
    temporary_root: None,
    detail: "Open a workspace before native sandbox policy can be applied.".to_string(),
  }
}

pub fn native_sandbox_status(policy: &SandboxPolicy) -> NativeSandboxStatus {
  let available = native_sandbox_available();
  let backend = native_backend_name().to_string();
  let active = cfg!(target_os = "macos") && available;
  let configured_temporary_root = policy
    .temporary_root()
    .map(|path| path.display().to_string());
  let temporary_root = if active {
    configured_temporary_root
  } else {
    None
  };
  let detail = if active {
    if let Some(temporary_root) = &temporary_root {
      format!(
        "Shell actions run through macOS Seatbelt with read/write access limited to {} and temporary files routed to {}.",
        policy.workspace_root().display(),
        temporary_root
      )
    } else {
      format!(
        "Shell actions run through macOS Seatbelt with read/write access limited to {}.",
        policy.workspace_root().display()
      )
    }
  } else if cfg!(target_os = "macos") {
    "macOS native sandbox backend is unavailable; shell actions still use approvals, timeouts, and cleanup.".to_string()
  } else {
    "Native macOS sandbox is unavailable on this platform; shell actions still use approvals, timeouts, and cleanup.".to_string()
  };

  NativeSandboxStatus {
    mode: "workspaceReadWrite".to_string(),
    backend,
    available,
    active,
    temporary_root,
    detail,
  }
}

pub fn native_sandbox_available() -> bool {
  #[cfg(target_os = "macos")]
  {
    Path::new(MACOS_SANDBOX_EXEC_PATH).is_file()
  }

  #[cfg(not(target_os = "macos"))]
  {
    false
  }
}

pub fn native_backend_name() -> &'static str {
  if cfg!(target_os = "macos") {
    "macosSeatbelt"
  } else {
    "processOnly"
  }
}

pub fn macos_sandbox_exec_path() -> &'static str {
  MACOS_SANDBOX_EXEC_PATH
}

pub fn macos_seatbelt_profile(policy: &SandboxPolicy) -> String {
  let workspace_root = seatbelt_string(policy.workspace_root());
  let mut profile = vec![
    "(version 1)".to_string(),
    "(deny default)".to_string(),
    "(allow process*)".to_string(),
    "(allow signal (target self))".to_string(),
    "(allow sysctl-read)".to_string(),
    "(allow file-read-metadata (subpath \"/\"))".to_string(),
    "(allow file-read*".to_string(),
    "  (subpath \"/System\")".to_string(),
    "  (subpath \"/Library\")".to_string(),
    "  (subpath \"/usr\")".to_string(),
    "  (subpath \"/bin\")".to_string(),
    "  (subpath \"/sbin\")".to_string(),
    "  (subpath \"/etc\")".to_string(),
    "  (subpath \"/dev\")".to_string(),
    format!("  (subpath \"{workspace_root}\")"),
    ")".to_string(),
    "(allow file-write*".to_string(),
    format!("  (subpath \"{workspace_root}\")"),
    ")".to_string(),
  ];

  if policy.allow_network() {
    profile.push("(allow network*)".to_string());
  }

  profile.join("\n")
}

fn seatbelt_string(path: &Path) -> String {
  path
    .to_string_lossy()
    .replace('\\', "\\\\")
    .replace('"', "\\\"")
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn profile_limits_network_by_default() {
    let policy = SandboxPolicy::workspace_read_write("/Users/example/work");
    let profile = macos_seatbelt_profile(&policy);

    assert!(!profile.contains("network*"));
    assert!(profile.contains("(deny default)"));
    assert!(profile.contains("(subpath \"/Users/example/work\")"));
    assert!(!profile.contains("(subpath \"/tmp\")"));
    assert!(!profile.contains("(subpath \"/private/tmp\")"));
    assert!(!profile.contains("(subpath \"/var/tmp\")"));
  }

  #[test]
  fn profile_escapes_workspace_path() {
    let policy = SandboxPolicy::workspace_read_write("/tmp/Pith \"Demo\"");
    let profile = macos_seatbelt_profile(&policy);

    assert!(profile.contains("(subpath \"/tmp/Pith \\\"Demo\\\"\")"));
  }

  #[cfg(target_os = "macos")]
  #[test]
  fn status_reports_workspace_temporary_root() {
    let policy = SandboxPolicy::workspace_read_write("/Users/example/work")
      .with_temporary_root("/Users/example/work/.pith/sandbox-tmp");
    let status = native_sandbox_status(&policy);

    if status.active {
      assert_eq!(
        status.temporary_root.as_deref(),
        Some("/Users/example/work/.pith/sandbox-tmp")
      );
    } else {
      assert_eq!(status.temporary_root, None);
    }
  }

  #[cfg(not(target_os = "macos"))]
  #[test]
  fn status_reports_process_only_outside_macos() {
    let policy = SandboxPolicy::workspace_read_write("/workspace");
    let status = native_sandbox_status(&policy);

    assert_eq!(status.backend, "processOnly");
    assert!(!status.available);
    assert!(!status.active);
    assert_eq!(status.temporary_root, None);
  }
}
