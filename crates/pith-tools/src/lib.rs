use std::collections::HashMap;
use std::fs;
use std::io::{ErrorKind, Read};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BuiltInTool {
  ReadFile,
  WriteFile,
  ListDirectory,
  SearchFiles,
  RunShell,
  GenerateDiff,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectoryEntry {
  pub name: String,
  pub relative_path: String,
  pub entry_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadFileResult {
  pub relative_path: String,
  pub content: String,
  pub is_truncated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchMatch {
  pub relative_path: String,
  pub line_number: usize,
  pub line: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellCommandResult {
  pub command: String,
  pub exit_code: i32,
  pub stdout: String,
  pub stderr: String,
  pub was_truncated: bool,
  pub timed_out: bool,
  pub sandbox: ShellSandboxSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellSandboxSummary {
  pub mode: String,
  pub backend: String,
  pub active: bool,
  pub temporary_root: Option<String>,
  pub detail: String,
}

impl ShellSandboxSummary {
  pub fn state(&self) -> &'static str {
    if self.active {
      "active"
    } else {
      "limited"
    }
  }

  pub fn display_line(&self) -> String {
    format!(
      "Sandbox: {} via {} ({})",
      self.mode,
      self.backend,
      self.state()
    )
  }

  pub fn attributes(&self) -> HashMap<String, String> {
    let mut attributes = HashMap::from([
      ("sandboxMode".to_string(), self.mode.clone()),
      ("sandboxBackend".to_string(), self.backend.clone()),
      ("sandboxActive".to_string(), self.active.to_string()),
      ("sandboxDetail".to_string(), self.detail.clone()),
    ]);
    if let Some(temporary_root) = &self.temporary_root {
      attributes.insert("sandboxTempRoot".to_string(), temporary_root.clone());
    }
    attributes
  }
}

const SHELL_COMMAND_TIMEOUT: Duration = Duration::from_secs(120);
const SHELL_POLL_INTERVAL: Duration = Duration::from_millis(50);
const SHELL_SANDBOX_TEMP_DIR: &str = ".pith/sandbox-tmp";

pub fn shell_command_timeout_seconds() -> u64 {
  SHELL_COMMAND_TIMEOUT.as_secs()
}

pub fn list_directory(
  workspace_root: &Path,
  relative_path: Option<&str>,
  limit: usize,
) -> Result<Vec<DirectoryEntry>> {
  let target = resolve_workspace_path(workspace_root, relative_path.unwrap_or("."), true)?;
  let workspace_root = fs::canonicalize(workspace_root).with_context(|| {
    format!(
      "failed to resolve workspace root {}",
      workspace_root.display()
    )
  })?;

  let mut entries = fs::read_dir(&target)
    .with_context(|| format!("failed to read directory {}", target.display()))?
    .filter_map(|entry| entry.ok())
    .map(|entry| {
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

      Ok(DirectoryEntry {
        name: entry.file_name().to_string_lossy().into_owned(),
        relative_path,
        entry_type: entry_type.to_string(),
      })
    })
    .collect::<Result<Vec<_>>>()?;

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
  let workspace_root = fs::canonicalize(workspace_root).with_context(|| {
    format!(
      "failed to resolve workspace root {}",
      workspace_root.display()
    )
  })?;
  let bytes =
    fs::read(&target).with_context(|| format!("failed to read file {}", target.display()))?;
  let is_truncated = bytes.len() > max_bytes;
  let preview_bytes = if is_truncated {
    &bytes[..max_bytes]
  } else {
    &bytes[..]
  };

  Ok(ReadFileResult {
    relative_path: relative_path_string(&workspace_root, &target)?,
    content: String::from_utf8_lossy(preview_bytes).into_owned(),
    is_truncated,
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
    validate_existing_workspace_path(&workspace_root, parent)?;
  }

  validate_workspace_write_target(&workspace_root, &target)?;
  if target.is_dir() {
    bail!("workspace path points to a directory");
  }

  fs::write(&target, content)
    .with_context(|| format!("failed to write file {}", target.display()))?;

  Ok(sanitized_relative_path)
}

pub fn search_files(
  workspace_root: &Path,
  query: &str,
  max_results: usize,
) -> Result<Vec<SearchMatch>> {
  let workspace_root = canonical_workspace_root(workspace_root)?;
  let normalized_query = query.trim().to_lowercase();

  if normalized_query.is_empty() {
    bail!("search query must not be empty");
  }

  let mut results = vec![];
  visit_directory(
    &workspace_root,
    &workspace_root,
    &normalized_query,
    max_results,
    &mut results,
  )?;

  Ok(results)
}

pub fn run_shell(
  workspace_root: &Path,
  command: &str,
  max_output_bytes: usize,
) -> Result<ShellCommandResult> {
  let workspace_root = canonical_workspace_root(workspace_root)?;
  let trimmed_command = command.trim();
  if trimmed_command.is_empty() {
    bail!("shell command must not be empty");
  }
  let sandbox = shell_sandbox_summary(&workspace_root);
  prepare_shell_sandbox_environment(&workspace_root, &sandbox)?;

  let output = run_shell_with_timeout(trimmed_command, &workspace_root, SHELL_COMMAND_TIMEOUT)
    .with_context(|| {
      format!(
        "failed to run shell command in {}",
        workspace_root.display()
      )
    })?;

  let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
  let mut stderr = String::from_utf8_lossy(&output.stderr).into_owned();
  if output.timed_out {
    let timeout_message = format!(
      "Command timed out after {} seconds and was terminated.",
      SHELL_COMMAND_TIMEOUT.as_secs()
    );
    if stderr.trim().is_empty() {
      stderr = timeout_message;
    } else {
      stderr = format!("{stderr}\n{timeout_message}");
    }
  }
  let combined_len = stdout.len() + stderr.len();
  let was_truncated = combined_len > max_output_bytes * 2;

  Ok(ShellCommandResult {
    command: trimmed_command.to_string(),
    exit_code: output.exit_code,
    stdout: truncate_output(&stdout, max_output_bytes),
    stderr: truncate_output(&stderr, max_output_bytes),
    was_truncated,
    timed_out: output.timed_out,
    sandbox,
  })
}

pub fn shell_sandbox_summary(workspace_root: &Path) -> ShellSandboxSummary {
  let status = shell_sandbox_status(workspace_root);

  ShellSandboxSummary {
    mode: status.mode,
    backend: status.backend,
    active: status.active,
    temporary_root: status.temporary_root,
    detail: status.detail,
  }
}

pub fn shell_sandbox_status(workspace_root: &Path) -> pith_sandbox::NativeSandboxStatus {
  let policy = shell_sandbox_policy(workspace_root);
  pith_sandbox::native_sandbox_status(&policy)
}

fn shell_sandbox_policy(workspace_root: &Path) -> pith_sandbox::SandboxPolicy {
  pith_sandbox::SandboxPolicy::workspace_read_write(workspace_root)
    .with_temporary_root(shell_sandbox_temp_root(workspace_root))
}

fn shell_sandbox_temp_root(workspace_root: &Path) -> PathBuf {
  workspace_root.join(SHELL_SANDBOX_TEMP_DIR)
}

fn prepare_shell_sandbox_environment(
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

fn run_shell_with_timeout(
  command: &str,
  workspace_root: &Path,
  timeout: Duration,
) -> Result<ShellOutput> {
  let mut child = build_shell_command(command, workspace_root)
    .current_dir(workspace_root)
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn()?;
  let stdout_reader = child.stdout.take().map(read_pipe_in_background);
  let stderr_reader = child.stderr.take().map(read_pipe_in_background);
  let started_at = Instant::now();
  let mut timed_out = false;

  let status = loop {
    if let Some(status) = child.try_wait()? {
      break status;
    }

    if started_at.elapsed() >= timeout {
      timed_out = true;
      terminate_shell_child(&mut child);
      break child.wait()?;
    }

    thread::sleep(SHELL_POLL_INTERVAL);
  };

  Ok(ShellOutput {
    exit_code: if timed_out {
      -1
    } else {
      status.code().unwrap_or(-1)
    },
    stdout: join_pipe_reader(stdout_reader),
    stderr: join_pipe_reader(stderr_reader),
    timed_out,
  })
}

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

  let previous_content = if target.is_file() {
    fs::read_to_string(&target)
      .with_context(|| format!("failed to read file {}", target.display()))?
  } else {
    String::new()
  };

  Ok(build_unified_diff(
    &sanitized_relative_path,
    &previous_content,
    next_content,
  ))
}

fn visit_directory(
  workspace_root: &Path,
  current_dir: &Path,
  normalized_query: &str,
  max_results: usize,
  results: &mut Vec<SearchMatch>,
) -> Result<()> {
  if results.len() >= max_results {
    return Ok(());
  }

  let mut entries = fs::read_dir(current_dir)
    .with_context(|| format!("failed to read directory {}", current_dir.display()))?
    .filter_map(|entry| entry.ok())
    .collect::<Vec<_>>();
  entries.sort_by_key(|entry| entry.path());

  for entry in entries {
    if results.len() >= max_results {
      break;
    }

    let path = entry.path();
    let metadata = fs::symlink_metadata(&path)
      .with_context(|| format!("failed to read metadata for {}", path.display()))?;
    if metadata.file_type().is_symlink() {
      continue;
    }

    if metadata.is_dir() {
      let resolved_directory = fs::canonicalize(&path)
        .with_context(|| format!("failed to resolve directory {}", path.display()))?;
      if !resolved_directory.starts_with(workspace_root) {
        continue;
      }
      visit_directory(
        workspace_root,
        &resolved_directory,
        normalized_query,
        max_results,
        results,
      )?;
      continue;
    }

    if !metadata.is_file() || metadata.len() > 256 * 1024 {
      continue;
    }

    let content = match fs::read(&path) {
      Ok(content) => content,
      Err(_) => continue,
    };
    if content.contains(&0) {
      continue;
    }

    let text = String::from_utf8_lossy(&content);
    for (index, line) in text.lines().enumerate() {
      if !line.to_lowercase().contains(normalized_query) {
        continue;
      }

      results.push(SearchMatch {
        relative_path: relative_path_string(workspace_root, &path)?,
        line_number: index + 1,
        line: line.trim().to_string(),
      });

      if results.len() >= max_results {
        break;
      }
    }
  }

  Ok(())
}

fn resolve_workspace_path(
  workspace_root: &Path,
  relative_path: &str,
  allow_directory: bool,
) -> Result<PathBuf> {
  let workspace_root = fs::canonicalize(workspace_root).with_context(|| {
    format!(
      "failed to resolve workspace root {}",
      workspace_root.display()
    )
  })?;
  let candidate = workspace_root.join(relative_path);
  let resolved = fs::canonicalize(&candidate)
    .with_context(|| format!("failed to resolve workspace path {}", candidate.display()))?;

  if !resolved.starts_with(&workspace_root) {
    bail!("workspace path escapes the selected workspace");
  }

  let metadata = fs::metadata(&resolved)
    .with_context(|| format!("failed to read metadata for {}", resolved.display()))?;

  if metadata.is_dir() && !allow_directory {
    bail!("workspace path points to a directory");
  }

  Ok(resolved)
}

fn relative_path_string(workspace_root: &Path, target: &Path) -> Result<String> {
  let relative = target
    .strip_prefix(workspace_root)
    .with_context(|| format!("failed to relativize {}", target.display()))?;

  if relative.as_os_str().is_empty() {
    return Ok(".".to_string());
  }

  Ok(relative.to_string_lossy().replace('\\', "/"))
}

fn canonical_workspace_root(workspace_root: &Path) -> Result<PathBuf> {
  fs::canonicalize(workspace_root).with_context(|| {
    format!(
      "failed to resolve workspace root {}",
      workspace_root.display()
    )
  })
}

fn validate_workspace_write_target(workspace_root: &Path, target: &Path) -> Result<()> {
  match fs::symlink_metadata(target) {
    Ok(metadata) => {
      if metadata.file_type().is_symlink() {
        bail!("workspace path points to a symlink");
      }
      validate_existing_workspace_path(workspace_root, target)?;
      validate_no_symlink_components(workspace_root, target)?;
    }
    Err(error) if error.kind() == ErrorKind::NotFound => {
      if let Some(parent) = target.parent() {
        validate_workspace_write_parent(workspace_root, parent)?;
      }
    }
    Err(error) => {
      return Err(error)
        .with_context(|| format!("failed to read metadata for {}", target.display()));
    }
  }

  Ok(())
}

fn validate_workspace_write_parent(workspace_root: &Path, parent: &Path) -> Result<()> {
  let existing_parent = nearest_existing_ancestor(parent).with_context(|| {
    format!(
      "failed to locate an existing parent for {}",
      parent.display()
    )
  })?;
  validate_existing_workspace_path(workspace_root, &existing_parent)?;
  validate_no_symlink_components(workspace_root, &existing_parent)
}

fn validate_existing_workspace_path(workspace_root: &Path, path: &Path) -> Result<()> {
  let resolved = fs::canonicalize(path)
    .with_context(|| format!("failed to resolve workspace path {}", path.display()))?;
  if !resolved.starts_with(workspace_root) {
    bail!("workspace path escapes the selected workspace");
  }

  Ok(())
}

fn validate_no_symlink_components(workspace_root: &Path, path: &Path) -> Result<()> {
  let relative = path
    .strip_prefix(workspace_root)
    .with_context(|| format!("failed to relativize {}", path.display()))?;
  let mut candidate = workspace_root.to_path_buf();

  for component in relative.components() {
    let std::path::Component::Normal(segment) = component else {
      continue;
    };
    candidate.push(segment);
    match fs::symlink_metadata(&candidate) {
      Ok(metadata) if metadata.file_type().is_symlink() => {
        bail!("workspace path crosses a symlink");
      }
      Ok(_) => {}
      Err(error) if error.kind() == ErrorKind::NotFound => return Ok(()),
      Err(error) => {
        return Err(error)
          .with_context(|| format!("failed to read metadata for {}", candidate.display()));
      }
    }
  }

  Ok(())
}

fn nearest_existing_ancestor(path: &Path) -> Option<PathBuf> {
  let mut candidate = path.to_path_buf();
  loop {
    if candidate.exists() {
      return Some(candidate);
    }
    if !candidate.pop() {
      return None;
    }
  }
}

fn sanitize_relative_path(relative_path: &str) -> Result<String> {
  let path = Path::new(relative_path);
  if path.is_absolute() {
    bail!("workspace path must be relative");
  }

  let mut sanitized = PathBuf::new();
  for component in path.components() {
    match component {
      std::path::Component::CurDir => {}
      std::path::Component::Normal(segment) => sanitized.push(segment),
      _ => bail!("workspace path must stay inside the selected workspace"),
    }
  }

  if sanitized.as_os_str().is_empty() {
    bail!("workspace path must not be empty");
  }

  Ok(sanitized.to_string_lossy().replace('\\', "/"))
}

struct ShellOutput {
  exit_code: i32,
  stdout: Vec<u8>,
  stderr: Vec<u8>,
  timed_out: bool,
}

fn read_pipe_in_background<R>(mut reader: R) -> thread::JoinHandle<Vec<u8>>
where
  R: Read + Send + 'static,
{
  thread::spawn(move || {
    let mut bytes = vec![];
    let _ = reader.read_to_end(&mut bytes);
    bytes
  })
}

fn join_pipe_reader(reader: Option<thread::JoinHandle<Vec<u8>>>) -> Vec<u8> {
  reader
    .and_then(|handle| handle.join().ok())
    .unwrap_or_default()
}

fn terminate_shell_child(child: &mut Child) {
  #[cfg(unix)]
  {
    terminate_unix_process_group(child);
  }

  #[cfg(not(unix))]
  {
    let _ = child.kill();
  }
}

#[cfg(unix)]
fn terminate_unix_process_group(child: &mut Child) {
  let process_group_id = -(child.id() as i32);
  unsafe {
    kill(process_group_id, SIGTERM);
  }
  thread::sleep(Duration::from_millis(200));
  if matches!(child.try_wait(), Ok(None)) {
    unsafe {
      kill(process_group_id, SIGKILL);
    }
  }
}

#[cfg(target_family = "windows")]
fn build_shell_command(command: &str, _workspace_root: &Path) -> Command {
  let mut process = Command::new("powershell");
  process.args(["-NoProfile", "-Command", command]);
  process
}

#[cfg(target_os = "macos")]
fn build_shell_command(command: &str, workspace_root: &Path) -> Command {
  if pith_sandbox::native_sandbox_available() {
    let policy = shell_sandbox_policy(workspace_root);
    let profile = pith_sandbox::macos_seatbelt_profile(&policy);
    let temporary_root = shell_sandbox_temp_root(workspace_root);
    let mut process = Command::new(pith_sandbox::macos_sandbox_exec_path());
    process
      .arg("-p")
      .arg(profile)
      .arg("/bin/sh")
      .arg("-lc")
      .arg(command)
      .env("TMPDIR", &temporary_root)
      .env("TMP", &temporary_root)
      .env("TEMP", &temporary_root);
    set_unix_process_group(&mut process);
    return process;
  }

  build_unix_shell_command(command)
}

#[cfg(all(unix, not(target_os = "macos")))]
fn build_shell_command(command: &str, _workspace_root: &Path) -> Command {
  build_unix_shell_command(command)
}

#[cfg(unix)]
fn build_unix_shell_command(command: &str) -> Command {
  let mut process = Command::new("sh");
  process.args(["-lc", command]);
  set_unix_process_group(&mut process);
  process
}

#[cfg(unix)]
fn set_unix_process_group(process: &mut Command) {
  use std::os::unix::process::CommandExt;

  unsafe {
    process.pre_exec(|| {
      if setpgid(0, 0) == 0 {
        Ok(())
      } else {
        Err(std::io::Error::last_os_error())
      }
    });
  }
}

#[cfg(unix)]
const SIGTERM: i32 = 15;
#[cfg(unix)]
const SIGKILL: i32 = 9;

#[cfg(unix)]
extern "C" {
  fn kill(pid: i32, sig: i32) -> i32;
  fn setpgid(pid: i32, pgid: i32) -> i32;
}

fn truncate_output(output: &str, max_output_bytes: usize) -> String {
  let mut collected = String::new();

  for character in output.chars() {
    if collected.len() + character.len_utf8() > max_output_bytes {
      break;
    }
    collected.push(character);
  }

  collected
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
  use std::time::{Duration, SystemTime, UNIX_EPOCH};

  use super::*;

  #[cfg(unix)]
  #[test]
  fn shell_timeout_terminates_blocking_command() {
    let workspace = unique_temp_workspace("shell-timeout");
    fs::create_dir_all(&workspace).expect("workspace");

    let result = run_shell_with_timeout("sleep 5", &workspace, Duration::from_millis(100))
      .expect("shell result");

    assert!(result.timed_out);
    assert_eq!(result.exit_code, -1);

    let _ = fs::remove_dir_all(workspace);
  }

  #[cfg(unix)]
  #[test]
  fn shell_result_reports_sandbox_summary() {
    let workspace = unique_temp_workspace("shell-sandbox");
    fs::create_dir_all(&workspace).expect("workspace");

    let result = run_shell(&workspace, "printf pith", 1024).expect("shell result");

    assert_eq!(result.stdout, "pith");
    assert_eq!(result.sandbox.mode, "workspaceReadWrite");
    assert!(!result.sandbox.backend.is_empty());
    let expected_temp_root = workspace.join(SHELL_SANDBOX_TEMP_DIR).display().to_string();
    if result.sandbox.active {
      assert_eq!(
        result.sandbox.temporary_root.as_deref(),
        Some(expected_temp_root.as_str())
      );
    } else {
      assert_eq!(result.sandbox.temporary_root, None);
    }
    assert!(!result.sandbox.detail.is_empty());

    let _ = fs::remove_dir_all(workspace);
  }

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

  #[cfg(unix)]
  #[test]
  fn search_files_skips_symlinked_directories() {
    use std::os::unix::fs::symlink;

    let workspace = unique_temp_workspace("search-symlink");
    let outside = unique_temp_workspace("search-outside");
    fs::create_dir_all(&workspace).expect("workspace");
    fs::create_dir_all(&outside).expect("outside");
    fs::write(workspace.join("inside.txt"), "visible needle").expect("inside file");
    fs::write(outside.join("secret.txt"), "hidden needle").expect("outside file");
    symlink(&outside, workspace.join("outside-link")).expect("symlink");

    let matches = search_files(&workspace, "needle", 10).expect("search");

    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].relative_path, "inside.txt");

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
