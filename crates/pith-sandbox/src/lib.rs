use std::fs;
use std::io::ErrorKind;
use std::path::{Component, Path, PathBuf};

#[cfg(target_os = "macos")]
use std::process::{Command, Stdio};
#[cfg(target_os = "macos")]
use std::sync::OnceLock;
#[cfg(target_os = "macos")]
use std::time::Duration;

#[cfg(target_os = "macos")]
use pith_process::{configure_process_group, wait_for_child, ChildExitReason};

const MACOS_SANDBOX_EXEC_PATH: &str = "/usr/bin/sandbox-exec";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SandboxPolicy {
  workspace_root: PathBuf,
  temporary_root: Option<PathBuf>,
  read_only_roots: Vec<PathBuf>,
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

impl NativeSandboxStatus {
  pub fn network_policy(&self) -> &'static str {
    network_policy_label(self.active, self.network_allowed)
  }
}

pub fn network_policy_label(active: bool, network_allowed: bool) -> &'static str {
  if active && network_allowed {
    "network allowed"
  } else if network_allowed {
    "network allowed by policy, not native-enforced"
  } else if active {
    "network denied"
  } else {
    "network denied by policy, not native-enforced"
  }
}

impl SandboxPolicy {
  pub fn workspace_read_write(workspace_root: impl Into<PathBuf>) -> Self {
    Self {
      workspace_root: workspace_root.into(),
      temporary_root: None,
      read_only_roots: vec![],
      allow_network: false,
    }
  }

  pub fn with_temporary_root(mut self, temporary_root: impl Into<PathBuf>) -> Self {
    self.temporary_root = Some(temporary_root.into());
    self
  }

  pub fn with_read_only_root(mut self, read_only_root: impl Into<PathBuf>) -> Self {
    self.read_only_roots.push(read_only_root.into());
    self
  }

  pub fn with_network_access(mut self, allow_network: bool) -> Self {
    self.allow_network = allow_network;
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

  pub fn read_only_roots(&self) -> Vec<&Path> {
    self.read_only_roots.iter().map(PathBuf::as_path).collect()
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
  let temporary_root = policy
    .temporary_root()
    .map(|path| path.display().to_string());
  let writable_roots = policy
    .writable_roots()
    .into_iter()
    .map(|path| path.display().to_string())
    .collect();
  let boundary_detail = sandbox_boundary_detail(policy, temporary_root.as_deref());
  let network_detail = if active {
    if policy.allow_network() {
      "Network access is allowed by policy."
    } else {
      "Network access is denied by default."
    }
  } else if policy.allow_network() {
    "Configured policy allows network access, but it is not native-enforced while inactive."
  } else {
    "Configured policy denies network access, but it is not native-enforced while inactive."
  };
  let detail = if active {
    format!("Local actions run through macOS Seatbelt with {boundary_detail}. {network_detail}")
  } else if cfg!(target_os = "macos") {
    format!(
      "macOS native sandbox backend is unavailable; configured policy targets {boundary_detail}. \
       Local actions still use approvals, workspace temp routing, timeouts, and cleanup. \
       {network_detail}"
    )
  } else {
    format!(
      "Native macOS sandbox is unavailable on this platform; configured policy targets \
       {boundary_detail}. Local actions still use approvals, workspace temp routing, timeouts, \
       and cleanup. {network_detail}"
    )
  };

  NativeSandboxStatus {
    mode: "workspaceReadWrite".to_string(),
    backend,
    available,
    active,
    network_allowed: policy.allow_network(),
    temporary_root,
    writable_roots,
    detail,
  }
}

fn sandbox_boundary_detail(policy: &SandboxPolicy, temporary_root: Option<&str>) -> String {
  if let Some(temporary_root) = temporary_root {
    format!(
      "write access limited to workspace {} and temporary files routed to {}{}",
      policy.workspace_root().display(),
      temporary_root,
      read_only_boundary_detail(policy)
    )
  } else {
    format!(
      "write access limited to workspace {}{}",
      policy.workspace_root().display(),
      read_only_boundary_detail(policy)
    )
  }
}

fn read_only_boundary_detail(policy: &SandboxPolicy) -> String {
  let roots = policy
    .read_only_roots()
    .into_iter()
    .map(|root| root.display().to_string())
    .collect::<Vec<_>>();
  if roots.is_empty() {
    String::new()
  } else {
    format!(" with read-only access to {}", roots.join(", "))
  }
}

pub fn native_sandbox_available() -> bool {
  #[cfg(target_os = "macos")]
  {
    static AVAILABLE: OnceLock<bool> = OnceLock::new();
    *AVAILABLE.get_or_init(probe_macos_sandbox_exec)
  }

  #[cfg(not(target_os = "macos"))]
  {
    false
  }
}

#[cfg(target_os = "macos")]
fn probe_macos_sandbox_exec() -> bool {
  if !Path::new(MACOS_SANDBOX_EXEC_PATH).is_file() {
    return false;
  }

  let mut command = Command::new(MACOS_SANDBOX_EXEC_PATH);
  configure_process_group(&mut command);
  let Ok(mut child) = command
    .arg("-p")
    .arg(macos_sandbox_probe_profile())
    .arg("/bin/sh")
    .arg("-c")
    .arg("true")
    .stdin(Stdio::null())
    .stdout(Stdio::null())
    .stderr(Stdio::null())
    .spawn()
  else {
    return false;
  };

  wait_for_child(
    &mut child,
    Duration::from_secs(2),
    Duration::from_millis(20),
    Duration::from_millis(50),
    || false,
  )
  .is_ok_and(|wait| wait.reason == ChildExitReason::Completed && wait.status.success())
}

#[cfg(target_os = "macos")]
fn macos_sandbox_probe_profile() -> &'static str {
  r#"(version 1)
(deny default)
(allow process*)
(allow signal (target self))
(allow sysctl-read)
(allow file-read-metadata (subpath "/"))
(allow file-read*
  (subpath "/System")
  (subpath "/Library")
  (subpath "/usr")
  (subpath "/bin")
  (subpath "/sbin")
  (subpath "/etc")
  (subpath "/private/etc")
  (subpath "/private/var/db")
  (subpath "/dev"))
(allow file-write-data (literal "/dev/null"))"#
}

pub fn native_backend_name() -> &'static str {
  #[cfg(target_os = "macos")]
  {
    if native_sandbox_available() {
      "macosSeatbelt"
    } else {
      "processOnly"
    }
  }

  #[cfg(not(target_os = "macos"))]
  {
    "processOnly"
  }
}

pub fn macos_sandbox_exec_path() -> &'static str {
  MACOS_SANDBOX_EXEC_PATH
}

pub fn prepare_workspace_temporary_root(
  workspace_root: &Path,
  temporary_root: &Path,
) -> Result<(), String> {
  let input_workspace_root = workspace_root.to_path_buf();
  let workspace_root = workspace_root
    .canonicalize()
    .map_err(|error| format!("Sandbox workspace root could not be resolved: {error}"))?;
  let temporary_root =
    temporary_root_for_canonical_workspace(&input_workspace_root, &workspace_root, temporary_root);
  if !temporary_root.starts_with(&workspace_root) {
    return Err("Sandbox temporary root must stay inside the selected workspace.".to_string());
  }
  let parent = temporary_root
    .parent()
    .ok_or_else(|| "Sandbox temporary root does not have a parent directory.".to_string())?;

  ensure_workspace_directory(&workspace_root, parent)?;
  clear_workspace_temporary_root(&temporary_root)?;
  fs::create_dir(&temporary_root)
    .map_err(|error| format!("Sandbox temporary directory could not be created: {error}"))?;
  ensure_workspace_directory(&workspace_root, &temporary_root)
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
  for read_only_root in policy.read_only_roots().into_iter().map(seatbelt_string) {
    if !readable_roots.contains(&read_only_root) {
      readable_roots.push(read_only_root);
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
    "  (subpath \"/private/etc\")".to_string(),
    "  (subpath \"/private/var/db\")".to_string(),
    "  (subpath \"/dev\")".to_string(),
  ];
  for readable_root in &readable_roots {
    profile.push(format!("  (subpath \"{readable_root}\")"));
  }
  profile.push(")".to_string());
  profile.push("(allow file-write-data (literal \"/dev/null\"))".to_string());
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

fn ensure_workspace_directory(workspace_root: &Path, directory: &Path) -> Result<(), String> {
  let relative = directory
    .strip_prefix(workspace_root)
    .map_err(|_| "Sandbox temporary path escapes the selected workspace.".to_string())?;
  let mut current = workspace_root.to_path_buf();

  for component in relative.components() {
    match component {
      Component::CurDir => continue,
      Component::Normal(segment) => current.push(segment),
      _ => {
        return Err("Sandbox temporary path must stay inside the selected workspace.".to_string());
      }
    }

    match fs::symlink_metadata(&current) {
      Ok(metadata) if metadata.file_type().is_symlink() => {
        return Err("Sandbox temporary path crosses a symlink.".to_string());
      }
      Ok(metadata) if !metadata.is_dir() => {
        return Err(format!(
          "Sandbox temporary path component is not a directory: {}",
          current.display()
        ));
      }
      Ok(_) => {}
      Err(error) if error.kind() == ErrorKind::NotFound => {
        fs::create_dir(&current).map_err(|error| {
          format!("Sandbox temporary directory component could not be created: {error}")
        })?;
        let metadata = fs::symlink_metadata(&current).map_err(|error| {
          format!("Sandbox temporary directory component could not be inspected: {error}")
        })?;
        if metadata.file_type().is_symlink() {
          return Err("Sandbox temporary path crosses a symlink.".to_string());
        }
        if !metadata.is_dir() {
          return Err(format!(
            "Sandbox temporary path component is not a directory: {}",
            current.display()
          ));
        }
      }
      Err(error) => {
        return Err(format!(
          "Sandbox temporary directory component could not be inspected: {error}"
        ));
      }
    }
  }

  Ok(())
}

fn clear_workspace_temporary_root(temporary_root: &Path) -> Result<(), String> {
  let metadata = match fs::symlink_metadata(temporary_root) {
    Ok(metadata) => metadata,
    Err(error) if error.kind() == ErrorKind::NotFound => return Ok(()),
    Err(error) => {
      return Err(format!(
        "Sandbox temporary directory could not be inspected: {error}"
      ));
    }
  };

  let result = if metadata.file_type().is_symlink() || metadata.is_file() {
    fs::remove_file(temporary_root)
  } else if metadata.is_dir() {
    fs::remove_dir_all(temporary_root)
  } else {
    fs::remove_file(temporary_root)
  };

  result.map_err(|error| format!("Sandbox temporary directory could not be cleared: {error}"))
}

fn temporary_root_for_canonical_workspace(
  input_workspace_root: &Path,
  canonical_workspace_root: &Path,
  temporary_root: &Path,
) -> PathBuf {
  if temporary_root.starts_with(canonical_workspace_root) {
    return temporary_root.to_path_buf();
  }

  temporary_root
    .strip_prefix(input_workspace_root)
    .map(|relative| canonical_workspace_root.join(relative))
    .unwrap_or_else(|_| temporary_root.to_path_buf())
}

fn seatbelt_string(path: &Path) -> String {
  let mut escaped = String::new();
  for character in path.to_string_lossy().chars() {
    match character {
      '\\' => escaped.push_str("\\\\"),
      '"' => escaped.push_str("\\\""),
      '\n' => escaped.push_str("\\n"),
      '\r' => escaped.push_str("\\r"),
      '\t' => escaped.push_str("\\t"),
      character if character.is_control() => escaped.push('_'),
      character => escaped.push(character),
    }
  }
  escaped
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
  fn profile_allows_network_when_policy_requests_it() {
    let policy =
      SandboxPolicy::workspace_read_write("/Users/example/work").with_network_access(true);
    let profile = macos_seatbelt_profile(&policy);

    assert!(profile.contains("(allow network*)"));
  }

  #[test]
  fn profile_allows_read_only_macos_runtime_roots() {
    let policy = SandboxPolicy::workspace_read_write("/Users/example/work");
    let profile = macos_seatbelt_profile(&policy);

    assert!(profile.contains("(subpath \"/private/etc\")"));
    assert!(profile.contains("(subpath \"/private/var/db\")"));
    let write_section = profile
      .split("(allow file-write*")
      .nth(1)
      .expect("write section");
    assert!(!write_section.contains("(subpath \"/private/etc\")"));
    assert!(!write_section.contains("(subpath \"/private/var/db\")"));
  }

  #[test]
  fn profile_escapes_workspace_path() {
    let policy = SandboxPolicy::workspace_read_write("/tmp/Pith \"Demo\"");
    let profile = macos_seatbelt_profile(&policy);

    assert!(profile.contains("(subpath \"/tmp/Pith \\\"Demo\\\"\")"));
  }

  #[test]
  fn profile_escapes_control_characters_in_paths() {
    let policy = SandboxPolicy::workspace_read_write("/tmp/Pith\nDemo\tRoot");
    let profile = macos_seatbelt_profile(&policy);

    assert!(!profile.contains("Pith\nDemo"));
    assert!(profile.contains("(subpath \"/tmp/Pith\\nDemo\\tRoot\")"));
  }

  #[test]
  fn profile_allows_temporary_root_explicitly() {
    let policy =
      SandboxPolicy::workspace_read_write("/Users/example/work").with_temporary_root("/tmp/pith");
    let profile = macos_seatbelt_profile(&policy);

    assert!(profile.contains("(subpath \"/Users/example/work\")"));
    assert!(profile.contains("(subpath \"/tmp/pith\")"));
  }

  #[test]
  fn profile_allows_read_only_roots_without_write_access() {
    let policy = SandboxPolicy::workspace_read_write("/Users/example/work")
      .with_read_only_root("/Users/example/plugin");
    let profile = macos_seatbelt_profile(&policy);

    assert!(profile.contains("(subpath \"/Users/example/plugin\")"));
    let write_section = profile
      .split("(allow file-write*")
      .nth(1)
      .expect("write section");
    assert!(!write_section.contains("(subpath \"/Users/example/plugin\")"));
  }

  #[test]
  fn profile_allows_dev_null_without_broad_dev_writes() {
    let policy = SandboxPolicy::workspace_read_write("/Users/example/work");
    let profile = macos_seatbelt_profile(&policy);

    assert!(profile.contains("(allow file-write-data (literal \"/dev/null\"))"));
    let write_section = profile
      .split("(allow file-write*")
      .nth(1)
      .expect("write section");
    assert!(!write_section.contains("(subpath \"/dev\")"));
  }

  #[test]
  fn prepare_workspace_temporary_root_clears_existing_directory() {
    let workspace = unique_temp_workspace("sandbox-temp-clear");
    let temporary_root = workspace.join(".pith/tmp");
    let stale_file = temporary_root.join("stale.txt");
    fs::create_dir_all(&temporary_root).expect("temporary root");
    fs::write(&stale_file, "stale").expect("stale file");

    prepare_workspace_temporary_root(&workspace, &temporary_root).expect("prepare temp root");

    assert!(temporary_root.is_dir());
    assert!(!stale_file.exists());

    let _ = fs::remove_dir_all(workspace);
  }

  #[cfg(unix)]
  #[test]
  fn prepare_workspace_temporary_root_removes_final_symlink_without_following() {
    use std::os::unix::fs::symlink;

    let workspace = unique_temp_workspace("sandbox-temp-symlink");
    let outside = unique_temp_workspace("sandbox-temp-outside");
    let temporary_root = workspace.join(".pith/tmp");
    let outside_file = outside.join("keep.txt");
    fs::create_dir_all(temporary_root.parent().expect("temporary parent"))
      .expect("temporary parent");
    fs::create_dir_all(&outside).expect("outside");
    fs::write(&outside_file, "keep").expect("outside file");
    symlink(&outside, &temporary_root).expect("temporary root symlink");

    prepare_workspace_temporary_root(&workspace, &temporary_root).expect("prepare temp root");

    assert!(temporary_root.is_dir());
    assert!(outside_file.is_file());

    let _ = fs::remove_dir_all(workspace);
    let _ = fs::remove_dir_all(outside);
  }

  #[cfg(unix)]
  #[test]
  fn prepare_workspace_temporary_root_rejects_parent_symlink_escape() {
    use std::os::unix::fs::symlink;

    let workspace = unique_temp_workspace("sandbox-temp-parent-symlink");
    let outside = unique_temp_workspace("sandbox-temp-parent-outside");
    let temporary_root = workspace.join(".pith/tmp/plugin");
    let parent = temporary_root.parent().expect("temporary parent");
    fs::create_dir_all(parent.parent().expect("pith directory")).expect("pith directory");
    fs::create_dir_all(&outside).expect("outside");
    symlink(&outside, parent).expect("temporary parent symlink");

    let error = prepare_workspace_temporary_root(&workspace, &temporary_root)
      .expect_err("parent symlink should fail");

    assert!(error.contains("crosses a symlink"));
    assert!(!outside.join("plugin").exists());

    let _ = fs::remove_dir_all(workspace);
    let _ = fs::remove_dir_all(outside);
  }

  #[cfg(target_os = "macos")]
  #[test]
  fn status_reports_workspace_temporary_root() {
    let policy = SandboxPolicy::workspace_read_write("/Users/example/work")
      .with_temporary_root("/Users/example/work/.pith/sandbox-tmp");
    let status = native_sandbox_status(&policy);

    assert_eq!(
      status.temporary_root.as_deref(),
      Some("/Users/example/work/.pith/sandbox-tmp")
    );
    assert!(status
      .writable_roots
      .contains(&"/Users/example/work".to_string()));
    assert!(status
      .writable_roots
      .contains(&"/Users/example/work/.pith/sandbox-tmp".to_string()));
  }

  #[cfg(not(target_os = "macos"))]
  #[test]
  fn status_reports_process_only_outside_macos() {
    let policy =
      SandboxPolicy::workspace_read_write("/workspace").with_temporary_root("/workspace/tmp");
    let status = native_sandbox_status(&policy);

    assert_eq!(status.backend, "processOnly");
    assert!(!status.available);
    assert!(!status.active);
    assert!(!status.network_allowed);
    assert_eq!(
      status.network_policy(),
      "network denied by policy, not native-enforced"
    );
    assert_eq!(status.temporary_root.as_deref(), Some("/workspace/tmp"));
    assert!(status.writable_roots.contains(&"/workspace".to_string()));
    assert!(status
      .writable_roots
      .contains(&"/workspace/tmp".to_string()));
  }

  #[cfg(not(target_os = "macos"))]
  #[test]
  fn status_reports_network_allowed_policy_outside_macos() {
    let policy = SandboxPolicy::workspace_read_write("/workspace").with_network_access(true);
    let status = native_sandbox_status(&policy);

    assert!(!status.active);
    assert!(status.network_allowed);
    assert_eq!(
      status.network_policy(),
      "network allowed by policy, not native-enforced"
    );
  }

  fn unique_temp_workspace(prefix: &str) -> PathBuf {
    let nonce = std::time::SystemTime::now()
      .duration_since(std::time::UNIX_EPOCH)
      .expect("clock")
      .as_nanos();
    std::env::temp_dir().join(format!("pith-sandbox-{prefix}-{nonce}"))
  }
}
