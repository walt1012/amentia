use std::collections::HashSet;
use std::fs::{create_dir_all, write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Result;

use crate::types::ShellOutputContext;

const SHELL_OUTPUT_ARTIFACT_DIR: &str = ".pith/sandbox-output";
const SHELL_OUTPUT_CONTEXT_MODE: &str = "sandboxOutputPreview";
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
  workspace_root: &Path,
  stdout: &str,
  stderr: &str,
  budget_bytes: usize,
  command: &str,
) -> ShellOutputContextResult {
  let artifact_directory =
    shell_output_artifact_directory(workspace_root, stdout, stderr, budget_bytes);
  let stdout_preview = compact_output_for_context(stdout, budget_bytes, command);
  let stderr_preview = compact_output_for_context(stderr, budget_bytes, command);
  let was_compacted = stdout_preview.len() < stdout.len() || stderr_preview.len() < stderr.len();

  ShellOutputContextResult {
    context: ShellOutputContext {
      mode: SHELL_OUTPUT_CONTEXT_MODE.to_string(),
      source_stdout_bytes: stdout.len(),
      source_stderr_bytes: stderr.len(),
      retained_stdout_bytes: stdout_preview.len(),
      retained_stderr_bytes: stderr_preview.len(),
      budget_bytes,
      was_compacted,
      artifact_directory,
    },
    stdout_preview,
    stderr_preview,
  }
}

fn compact_output_for_context(output: &str, budget_bytes: usize, command: &str) -> String {
  if output.len() <= budget_bytes {
    return output.to_string();
  }

  let important_lines = select_important_output_lines(output, command, budget_bytes / 2);
  let marker = format!(
    "\n[{} bytes omitted from sandbox output]\n",
    output.len().saturating_sub(budget_bytes)
  );
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

fn shell_output_artifact_directory(
  workspace_root: &Path,
  stdout: &str,
  stderr: &str,
  budget_bytes: usize,
) -> Option<String> {
  if stdout.len() <= budget_bytes && stderr.len() <= budget_bytes {
    return None;
  }

  write_shell_output_artifact(workspace_root, stdout, stderr).ok()
}

fn write_shell_output_artifact(
  workspace_root: &Path,
  stdout: &str,
  stderr: &str,
) -> Result<String> {
  let relative_directory = shell_output_artifact_relative_directory();
  let artifact_directory = workspace_root.join(&relative_directory);
  create_dir_all(&artifact_directory)?;
  write(artifact_directory.join("stdout.txt"), stdout)?;
  write(artifact_directory.join("stderr.txt"), stderr)?;
  Ok(relative_directory.to_string_lossy().replace('\\', "/"))
}

fn shell_output_artifact_relative_directory() -> PathBuf {
  let nonce = SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .expect("system time")
    .as_nanos();
  PathBuf::from(SHELL_OUTPUT_ARTIFACT_DIR).join(format!("run-{nonce}"))
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

    let preview = compact_output_for_context(&output, 120, "cat artifact.log");

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

    let preview = compact_output_for_context(&output, 180, "cat package.json");

    assert!(preview.contains("[selected sandbox output]"));
    assert!(preview.contains("error: failed to open package.json"));
    assert!(preview.len() <= 180);
  }

  #[test]
  fn shell_output_artifact_stores_full_output_inside_workspace() {
    let workspace = unique_temp_workspace("shell-output-artifact");
    fs::create_dir_all(&workspace).expect("workspace");

    let relative_directory =
      write_shell_output_artifact(&workspace, "full stdout", "full stderr").expect("artifact");
    let artifact_directory = workspace.join(&relative_directory);

    assert!(relative_directory.starts_with(".pith/sandbox-output/run-"));
    assert_eq!(
      fs::read_to_string(artifact_directory.join("stdout.txt")).expect("stdout"),
      "full stdout"
    );
    assert_eq!(
      fs::read_to_string(artifact_directory.join("stderr.txt")).expect("stderr"),
      "full stderr"
    );

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
