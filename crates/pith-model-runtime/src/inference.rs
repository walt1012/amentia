use std::env;
use std::path::Path;
use std::process::{Command, Output, Stdio};
use std::time::Duration;

use anyhow::{bail, Context, Result};
use pith_process::{
  configure_process_group, join_bounded_pipe_reader, read_bounded_pipe_in_background,
  wait_for_child, ChildExitReason,
};

use crate::{GenerateRequest, GenerationCancellation, ModelPackManifest, ModelRole};

const LLAMA_CPP_TIMEOUT: Duration = Duration::from_secs(180);
const LLAMA_CPP_POLL_INTERVAL: Duration = Duration::from_millis(50);
const LLAMA_CPP_PIPE_OUTPUT_LIMIT: usize = 4 * 1024 * 1024;

pub(crate) fn generate_with_llama_cpp(
  binary_path: &Path,
  model_path: &Path,
  request: &GenerateRequest,
  manifest: Option<&ModelPackManifest>,
) -> Result<String> {
  let context_size = manifest.map(|item| item.context_size).unwrap_or(4096);
  let max_tokens = manifest
    .map(|item| item.max_output_tokens.min(request.max_tokens))
    .unwrap_or(request.max_tokens);
  let output = run_llama_cpp_with_timeout(
    binary_path,
    model_path,
    context_size,
    max_tokens,
    &request.prompt,
    request.cancellation.as_ref(),
  )
  .with_context(|| format!("failed to execute {}", binary_path.display()))?;

  if !output.status.success() {
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    bail!(
      "llama.cpp exited with status {}: {}",
      output.status,
      if stderr.is_empty() {
        "no stderr output".to_string()
      } else {
        stderr
      }
    );
  }

  let text = String::from_utf8(output.stdout).context("llama.cpp output was not valid UTF-8")?;
  let cleaned = clean_model_output(&text);
  if cleaned.is_empty() {
    bail!("llama.cpp produced an empty response");
  }

  Ok(cleaned)
}

fn run_llama_cpp_with_timeout(
  binary_path: &Path,
  model_path: &Path,
  context_size: usize,
  max_tokens: usize,
  prompt: &str,
  cancellation: Option<&GenerationCancellation>,
) -> Result<Output> {
  let timeout = llama_cpp_timeout();
  let mut child = build_llama_cpp_command(binary_path)
    .arg("-m")
    .arg(model_path)
    .arg("--temp")
    .arg("0.2")
    .arg("--ctx-size")
    .arg(context_size.to_string())
    .arg("-n")
    .arg(max_tokens.to_string())
    .arg("--no-display-prompt")
    .arg("-p")
    .arg(prompt)
    .stdin(Stdio::null())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn()?;
  let stdout_reader = child
    .stdout
    .take()
    .map(|reader| read_bounded_pipe_in_background(reader, LLAMA_CPP_PIPE_OUTPUT_LIMIT));
  let stderr_reader = child
    .stderr
    .take()
    .map(|reader| read_bounded_pipe_in_background(reader, LLAMA_CPP_PIPE_OUTPUT_LIMIT));
  let wait = wait_for_child(
    &mut child,
    timeout,
    LLAMA_CPP_POLL_INTERVAL,
    Duration::from_millis(200),
    || {
      cancellation
        .map(GenerationCancellation::is_cancelled)
        .unwrap_or(false)
    },
  )?;

  let stdout = join_bounded_pipe_reader(stdout_reader).bytes;
  let stderr = join_bounded_pipe_reader(stderr_reader).bytes;

  if wait.reason == ChildExitReason::TimedOut {
    let stderr_text = String::from_utf8_lossy(&stderr).trim().to_string();
    if stderr_text.is_empty() {
      bail!("llama.cpp timed out after {} seconds", timeout.as_secs());
    }

    bail!(
      "llama.cpp timed out after {} seconds: {}",
      timeout.as_secs(),
      stderr_text
    );
  }

  Ok(Output {
    status: wait.status,
    stdout,
    stderr,
  })
}

pub(crate) fn request_is_cancelled(request: &GenerateRequest) -> bool {
  request
    .cancellation
    .as_ref()
    .map(GenerationCancellation::is_cancelled)
    .unwrap_or(false)
}

fn llama_cpp_timeout() -> Duration {
  env::var("PITH_LLAMA_CPP_TIMEOUT_SECONDS")
    .ok()
    .and_then(|value| value.parse::<u64>().ok())
    .filter(|seconds| *seconds > 0)
    .map(Duration::from_secs)
    .unwrap_or(LLAMA_CPP_TIMEOUT)
}

pub fn llama_cpp_timeout_seconds() -> u64 {
  llama_cpp_timeout().as_secs()
}

fn build_llama_cpp_command(binary_path: &Path) -> Command {
  let mut process = Command::new(binary_path);

  configure_process_group(&mut process);

  process
}

fn clean_model_output(output: &str) -> String {
  output
    .lines()
    .filter(|line| {
      let trimmed = line.trim();
      !trimmed.is_empty() && !trimmed.starts_with("build:") && !trimmed.starts_with("main:")
    })
    .collect::<Vec<_>>()
    .join("\n")
    .trim()
    .to_string()
}

pub(crate) fn generation_failure_text(role: &ModelRole, detail: &str) -> String {
  let role_label = match role {
    ModelRole::Default => "default",
    ModelRole::Planner => "planner",
    ModelRole::Coder => "coder",
    ModelRole::Summarizer => "summarizer",
  };

  format!("Pith could not produce a local {role_label} response because {detail}")
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::io::Cursor;

  #[test]
  fn llama_pipe_reader_bounds_retained_output() {
    let input = Cursor::new(vec![b'x'; LLAMA_CPP_PIPE_OUTPUT_LIMIT + 512]);
    let output = join_bounded_pipe_reader(Some(read_bounded_pipe_in_background(
      input,
      LLAMA_CPP_PIPE_OUTPUT_LIMIT,
    )));

    assert_eq!(output.bytes.len(), LLAMA_CPP_PIPE_OUTPUT_LIMIT);
    assert_eq!(output.source_byte_count, LLAMA_CPP_PIPE_OUTPUT_LIMIT + 512);
  }
}
