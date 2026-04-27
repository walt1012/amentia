use std::fs;
use std::io::Read;
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
  pub detail: String,
}

const SHELL_COMMAND_TIMEOUT: Duration = Duration::from_secs(120);
const SHELL_POLL_INTERVAL: Duration = Duration::from_millis(50);

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
      let metadata = entry
        .metadata()
        .with_context(|| format!("failed to read metadata for {}", path.display()))?;
      let entry_type = if metadata.is_dir() {
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
    fs::create_dir_all(parent)
      .with_context(|| format!("failed to create directory {}", parent.display()))?;
  }

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
  let status = pith_sandbox::native_sandbox_status(
    &pith_sandbox::SandboxPolicy::workspace_read_write(workspace_root),
  );

  ShellSandboxSummary {
    mode: status.mode,
    backend: status.backend,
    active: status.active,
    detail: status.detail,
  }
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
    let metadata = entry
      .metadata()
      .with_context(|| format!("failed to read metadata for {}", path.display()))?;

    if metadata.is_dir() {
      visit_directory(
        workspace_root,
        &path,
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
    let policy = pith_sandbox::SandboxPolicy::workspace_read_write(workspace_root);
    let profile = pith_sandbox::macos_seatbelt_profile(&policy);
    let mut process = Command::new(pith_sandbox::macos_sandbox_exec_path());
    process
      .arg("-p")
      .arg(profile)
      .arg("/bin/sh")
      .arg("-lc")
      .arg(command);
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
    assert!(!result.sandbox.detail.is_empty());

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
