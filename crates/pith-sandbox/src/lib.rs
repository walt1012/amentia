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
  pub network_allowed: bool,
  pub temporary_root: Option<String>,
  pub writable_roots: Vec<String>,
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

  pub fn writable_roots(&self) -> Vec<&Path> {
    let mut roots = vec![self.workspace_root()];
    if let Some(temporary_root) = self.temporary_root() {
      roots.push(temporary_root);
    }
    roots
  }
}

pub fn workspace_required_status() -> NativeSandboxStatus {
  NativeSandboxStatus {
    mode: "workspaceReadWrite".to_string(),
    backend: native_backend_name().to_string(),
    available: native_sandbox_available(),
    active: false,
    network_allowed: false,
    temporary_root: None,
    writable_roots: Vec::new(),
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
  let writable_roots = if active {
    policy
      .writable_roots()
      .into_iter()
      .map(|path| path.display().to_string())
      .collect()
  } else {
    Vec::new()
  };
  let temporary_root = if active {
    configured_temporary_root
  } else {
    None
  };
  let network_detail = if policy.allow_network() {
    "Network access is allowed by policy."
  } else {
    "Network access is denied by default."
  };
  let detail = if active {
    if let Some(temporary_root) = &temporary_root {
      format!(
        "Shell actions run through macOS Seatbelt with write access limited to workspace {} and temporary files routed to {}. {}",
        policy.workspace_root().display(),
        temporary_root,
        network_detail
      )
    } else {
      format!(
        "Shell actions run through macOS Seatbelt with write access limited to workspace {}. {}",
        policy.workspace_root().display(),
        network_detail
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
    network_allowed: active && policy.allow_network(),
    temporary_root,
    writable_roots,
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
  let writable_roots = policy
    .writable_roots()
    .into_iter()
    .map(seatbelt_string)
    .collect::<Vec<_>>();
  let mut readable_roots = vec![workspace_root];
  for writable_root in &writable_roots {
    if !readable_roots.contains(writable_root) {
      readable_roots.push(writable_root.clone());
    }
  }
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
  ];
  for readable_root in &readable_roots {
    profile.push(format!("  (subpath \"{readable_root}\")"));
  }
  profile.push(")".to_string());
  profile.push("(allow file-write*".to_string());
  for writable_root in &writable_roots {
    profile.push(format!("  (subpath \"{writable_root}\")"));
  }
  profile.push(")".to_string());

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

  #[test]
  fn profile_allows_temporary_root_explicitly() {
    let policy = SandboxPolicy::workspace_read_write("/Users/example/work")
      .with_temporary_root("/tmp/pith");
    let profile = macos_seatbelt_profile(&policy);

    assert!(profile.contains("(subpath \"/Users/example/work\")"));
    assert!(profile.contains("(subpath \"/tmp/pith\")"));
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
    assert!(!status.network_allowed);
    assert_eq!(status.temporary_root, None);
    assert!(status.writable_roots.is_empty());
  }
}
