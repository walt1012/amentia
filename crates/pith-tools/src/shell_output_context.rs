use std::collections::HashSet;
use std::env;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{bail, Context, Result};

use crate::types::ShellOutputContext;

const SHELL_OUTPUT_CONTEXT_MODE: &str = "sandboxOutputPreview";
const SHELL_OUTPUT_ARTIFACT_RETAINED_RUNS: usize = 20;
const OUTPUT_CONTEXT_MARKERS: [&str; 9] = [
  "error",
  "failed",
  "failure",
  "panic",
  "warning",
  "exception",
  "not found",
  "permission",
  "denied",
];

pub(crate) struct ShellOutputContextResult {
  pub(crate) stdout_preview: String,
  pub(crate) stderr_preview: String,
  pub(crate) context: ShellOutputContext,
}

pub(crate) fn build_shell_output_context(
  stdout: &str,
  stderr: &str,
  source_stdout_bytes: usize,
  source_stderr_bytes: usize,
  budget_bytes: usize,
  artifact_directory: Option<&Path>,
  command: &str,
) -> ShellOutputContextResult {
  let stdout_preview =
    compact_output_for_context(stdout, source_stdout_bytes, budget_bytes, command);
  let stderr_preview =
    compact_output_for_context(stderr, source_stderr_bytes, budget_bytes, command);
  let was_compacted =
    source_stdout_bytes > stdout_preview.len() || source_stderr_bytes > stderr_preview.len();

  ShellOutputContextResult {
    context: ShellOutputContext {
      mode: SHELL_OUTPUT_CONTEXT_MODE.to_string(),
      source_stdout_bytes,
      source_stderr_bytes,
      retained_stdout_bytes: stdout_preview.len(),
      retained_stderr_bytes: stderr_preview.len(),
      budget_bytes,
      was_compacted,
      artifact_directory: artifact_directory.map(|path| path.display().to_string()),
    },
    stdout_preview,
    stderr_preview,
  }
}

pub(crate) fn shell_output_artifact_directory() -> Result<PathBuf> {
  let root = shell_output_artifact_root();
  ensure_shell_output_artifact_root(&root)?;
  prune_shell_output_artifact_root(&root, SHELL_OUTPUT_ARTIFACT_RETAINED_RUNS);
  create_shell_output_artifact_run_directory(&root)
}

fn compact_output_for_context(
  output: &str,
  source_bytes: usize,
  budget_bytes: usize,
  command: &str,
) -> String {
  if source_bytes <= output.len() && output.len() <= budget_bytes {
    return output.to_string();
  }

  let important_lines = select_important_output_lines(output, command, budget_bytes / 2);
  let omitted_bytes = if source_bytes > output.len() {
    source_bytes.saturating_sub(output.len())
  } else {
    output.len().saturating_sub(budget_bytes)
  };
  let marker = format!("\n[{} bytes omitted from sandbox output]\n", omitted_bytes);
  let marker_len = marker.len();
  if budget_bytes <= marker_len {
    return truncate_output(&marker, budget_bytes);
  }

  if !important_lines.is_empty() {
    let remaining_budget = budget_bytes
      .saturating_sub(important_lines.len())
      .saturating_sub(marker_len);
    let tail = take_last_output(output, remaining_budget);
    return format!("{important_lines}{marker}{tail}");
  }

  let remaining_budget = budget_bytes.saturating_sub(marker_len);
  if output.len() <= remaining_budget {
    return format!("{output}{marker}");
  }

  let head_budget = (remaining_budget * 2) / 3;
  let tail_budget = remaining_budget.saturating_sub(head_budget);
  format!(
    "{}{}{}",
    truncate_output(output, head_budget),
    marker,
    take_last_output(output, tail_budget)
  )
}

fn select_important_output_lines(output: &str, command: &str, budget_bytes: usize) -> String {
  if budget_bytes == 0 {
    return String::new();
  }

  let command_tokens = command_token_set(command);
  let mut selected = String::new();
  for line in output
    .lines()
    .filter(|line| output_line_is_relevant(line, &command_tokens))
  {
    let candidate = if selected.is_empty() {
      format!("[selected sandbox output]\n{line}\n")
    } else {
      format!("{line}\n")
    };
    if selected.len() + candidate.len() > budget_bytes {
      break;
    }
    selected.push_str(&candidate);
  }
  selected
}

fn output_line_is_relevant(line: &str, command_tokens: &HashSet<String>) -> bool {
  let normalized_line = line.to_ascii_lowercase();
  OUTPUT_CONTEXT_MARKERS
    .iter()
    .any(|marker| normalized_line.contains(marker))
    || command_tokens
      .iter()
      .any(|token| normalized_line.contains(token))
}

fn command_token_set(command: &str) -> HashSet<String> {
  command
    .chars()
    .map(|character| {
      if character.is_alphanumeric() {
        character.to_ascii_lowercase()
      } else {
        ' '
      }
    })
    .collect::<String>()
    .split_whitespace()
    .filter(|token| token.len() > 2)
    .map(str::to_string)
    .collect()
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

fn take_last_output(output: &str, max_output_bytes: usize) -> String {
  let mut collected = String::new();

  for character in output.chars().rev() {
    if collected.len() + character.len_utf8() > max_output_bytes {
      break;
    }
    collected.insert(0, character);
  }

  collected
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
  fs::create_dir_all(root)
    .with_context(|| format!("failed to create shell output artifact root {}", root.display()))?;
  let metadata = fs::symlink_metadata(root)
    .with_context(|| format!("failed to inspect shell output artifact root {}", root.display()))?;
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
  fn shell_output_context_compacts_large_output_and_keeps_tail() {
    let output = format!("{}tail marker", "A".repeat(900));

    let preview = compact_output_for_context(&output, output.len(), 120, "cat artifact.log");

    assert!(preview.contains("bytes omitted from sandbox output"));
    assert!(preview.ends_with("tail marker"));
    assert!(preview.len() <= 120);
  }

  #[test]
  fn shell_output_context_keeps_relevant_error_lines() {
    let output = format!(
      "{}\nerror: failed to open package.json\n{}",
      "noise\n".repeat(80),
      "tail\n".repeat(80)
    );

    let preview = compact_output_for_context(&output, output.len(), 180, "cat package.json");

    assert!(preview.contains("[selected sandbox output]"));
    assert!(preview.contains("error: failed to open package.json"));
    assert!(preview.len() <= 180);
  }

  #[test]
  fn shell_output_context_reports_app_owned_artifact_path() {
    let artifact_directory = PathBuf::from("/tmp/pith-artifacts/run-1");

    let context = build_shell_output_context(
      "preview",
      "",
      4096,
      0,
      1024,
      Some(&artifact_directory),
      "cat package.json",
    )
    .context;

    assert!(context.was_compacted);
    assert_eq!(
      context.artifact_directory.as_deref(),
      Some("/tmp/pith-artifacts/run-1")
    );
  }

  #[test]
  fn shell_output_context_marks_stream_omissions_when_preview_was_bounded() {
    let context =
      build_shell_output_context("head preview", "", 4096, 0, 128, None, "cat log").context;

    assert!(context.was_compacted);
    assert_eq!(context.source_stdout_bytes, 4096);
    assert!(context.retained_stdout_bytes > "head preview".len());
    assert!(context.retained_stdout_bytes <= 128);
  }

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
