use std::collections::HashSet;
use std::path::Path;

use crate::types::ShellOutputContext;

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
  stdout: &str,
  stderr: &str,
  source_stdout_bytes: usize,
  source_stderr_bytes: usize,
  budget_bytes: usize,
  artifact_stdout_bytes: usize,
  artifact_stderr_bytes: usize,
  artifact_max_bytes_per_stream: usize,
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
      artifact_stdout_bytes,
      artifact_stderr_bytes,
      artifact_max_bytes_per_stream,
      artifacts_truncated: artifact_stdout_bytes < source_stdout_bytes
        || artifact_stderr_bytes < source_stderr_bytes,
      was_compacted,
      artifact_directory: artifact_directory.map(|path| path.display().to_string()),
    },
    stdout_preview,
    stderr_preview,
  }
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

#[cfg(test)]
mod tests {
  use std::path::PathBuf;

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
      4096,
      0,
      4096,
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
    let context = build_shell_output_context(
      "head preview",
      "",
      4096,
      0,
      128,
      4096,
      0,
      4096,
      None,
      "cat log",
    )
    .context;

    assert!(context.was_compacted);
    assert_eq!(context.source_stdout_bytes, 4096);
    assert!(context.retained_stdout_bytes > "head preview".len());
    assert!(context.retained_stdout_bytes <= 128);
  }
}
