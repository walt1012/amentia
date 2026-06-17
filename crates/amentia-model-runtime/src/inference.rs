use std::env;
use std::ffi::OsString;
use std::fs::{self, OpenOptions};
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::process::{self, Command, Output, Stdio};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{bail, Context, Result};
use amentia_process::{
  configure_process_group, join_bounded_pipe_reader, read_bounded_pipe_in_background,
  wait_for_child, ChildExitReason,
};

use crate::{GenerateRequest, GenerationCancellation, ModelPackManifest, ModelRole};

const LLAMA_CPP_TIMEOUT: Duration = Duration::from_secs(180);
const LLAMA_CPP_POLL_INTERVAL: Duration = Duration::from_millis(50);
const LLAMA_CPP_PIPE_OUTPUT_LIMIT: usize = 4 * 1024 * 1024;
const STALE_PROMPT_FILE_AGE: Duration = Duration::from_secs(15 * 60);

pub(crate) fn generate_with_llama_cpp(
  binary_path: &Path,
  model_path: &Path,
  request: &GenerateRequest,
  manifest: Option<&ModelPackManifest>,
) -> Result<String> {
  let context_size = manifest
    .map(|item| item.context_size.max(512))
    .unwrap_or(4096);
  let requested_max_tokens = request.max_tokens.max(1);
  let max_tokens = manifest
    .map(|item| item.max_output_tokens.max(1).min(requested_max_tokens))
    .unwrap_or(requested_max_tokens);
  let output = run_llama_cpp_with_timeout(
    binary_path,
    model_path,
    context_size,
    max_tokens,
    request.timeout,
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
  timeout: Option<Duration>,
  prompt: &str,
  cancellation: Option<&GenerationCancellation>,
) -> Result<Output> {
  let timeout = timeout.unwrap_or_else(llama_cpp_timeout);
  let prompt_file =
    PromptFile::create(prompt).context("failed to prepare llama.cpp prompt input")?;
  let mut child = build_llama_cpp_command(binary_path)
    .args(llama_cpp_arguments(
      model_path,
      prompt_file.path(),
      context_size,
      max_tokens,
    ))
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
  env::var("AMENTIA_LLAMA_CPP_TIMEOUT_SECONDS")
    .ok()
    .and_then(|value| value.parse::<u64>().ok())
    .filter(|seconds| *seconds > 0)
    .map(Duration::from_secs)
    .unwrap_or(LLAMA_CPP_TIMEOUT)
}

pub fn llama_cpp_timeout_seconds() -> u64 {
  llama_cpp_timeout().as_secs()
}

fn llama_cpp_arguments(
  model_path: &Path,
  prompt_path: &Path,
  context_size: usize,
  max_tokens: usize,
) -> Vec<OsString> {
  vec![
    OsString::from("-m"),
    model_path.as_os_str().to_os_string(),
    OsString::from("--temp"),
    OsString::from("0.2"),
    OsString::from("--ctx-size"),
    OsString::from(context_size.to_string()),
    OsString::from("-n"),
    OsString::from(max_tokens.to_string()),
    OsString::from("--no-display-prompt"),
    OsString::from("-f"),
    prompt_path.as_os_str().to_os_string(),
  ]
}

fn build_llama_cpp_command(binary_path: &Path) -> Command {
  let mut process = Command::new(binary_path);

  configure_process_group(&mut process);

  process
}

struct PromptFile {
  path: PathBuf,
}

impl PromptFile {
  fn create(prompt: &str) -> Result<Self> {
    let directory = env::temp_dir().join("amentia-model-prompts");
    fs::create_dir_all(&directory)
      .with_context(|| format!("failed to create {}", directory.display()))?;
    remove_stale_prompt_files(&directory);

    for attempt in 0..16 {
      let path = directory.join(format!(
        "prompt-{}-{}-{attempt}.txt",
        process::id(),
        prompt_file_nonce()
      ));

      match OpenOptions::new().write(true).create_new(true).open(&path) {
        Ok(mut file) => {
          if let Err(error) = file.write_all(prompt.as_bytes()) {
            let _ = fs::remove_file(&path);
            return Err(error).with_context(|| format!("failed to write {}", path.display()));
          }
          return Ok(Self { path });
        }
        Err(error) if error.kind() == ErrorKind::AlreadyExists => continue,
        Err(error) => {
          return Err(error).with_context(|| format!("failed to create {}", path.display()));
        }
      }
    }

    bail!(
      "failed to reserve a unique llama.cpp prompt file in {}",
      directory.display()
    )
  }

  fn path(&self) -> &Path {
    &self.path
  }
}

impl Drop for PromptFile {
  fn drop(&mut self) {
    let _ = fs::remove_file(&self.path);
  }
}

fn remove_stale_prompt_files(directory: &Path) {
  let now = SystemTime::now();
  let Ok(entries) = fs::read_dir(directory) else {
    return;
  };

  for entry in entries.flatten() {
    let path = entry.path();
    let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
      continue;
    };
    if !file_name.starts_with("prompt-") || !file_name.ends_with(".txt") {
      continue;
    }

    let Ok(metadata) = entry.metadata() else {
      continue;
    };
    if !metadata.is_file() {
      continue;
    }

    let Ok(modified) = metadata.modified() else {
      continue;
    };
    let is_stale = now
      .duration_since(modified)
      .map(|age| age >= STALE_PROMPT_FILE_AGE)
      .unwrap_or(false);
    if is_stale {
      let _ = fs::remove_file(path);
    }
  }
}

fn prompt_file_nonce() -> u128 {
  SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .map(|duration| duration.as_nanos())
    .unwrap_or(0)
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

  format!("Amentia could not produce a local {role_label} response because {detail}")
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

  #[test]
  fn llama_cpp_arguments_use_prompt_file_instead_of_inline_prompt() {
    let args = llama_cpp_arguments(
      std::path::Path::new("test.gguf"),
      std::path::Path::new("amentia-prompt.txt"),
      4096,
      128,
    )
    .into_iter()
    .map(|value| value.to_string_lossy().to_string())
    .collect::<Vec<_>>();

    assert!(args.contains(&"-f".to_string()));
    assert!(!args.contains(&"-p".to_string()));
    assert!(!args.contains(&"secret prompt".to_string()));
    assert_eq!(
      args
        .iter()
        .position(|value| value == "-f")
        .and_then(|index| args.get(index + 1)),
      Some(&"amentia-prompt.txt".to_string())
    );
  }

  #[test]
  fn prompt_file_removes_prompt_after_use() {
    let path = {
      let prompt_file = PromptFile::create("test prompt").expect("prompt file");
      let path = prompt_file.path().to_path_buf();

      assert_eq!(
        std::fs::read_to_string(&path).expect("prompt file content"),
        "test prompt"
      );

      path
    };

    assert!(!path.exists());
  }

  #[test]
  fn prompt_cleanup_keeps_fresh_prompt_file() {
    let prompt_file = PromptFile::create("fresh prompt").expect("prompt file");
    let path = prompt_file.path().to_path_buf();
    let directory = path.parent().expect("prompt directory").to_path_buf();

    remove_stale_prompt_files(&directory);

    assert_eq!(
      std::fs::read_to_string(&path).expect("prompt file content"),
      "fresh prompt"
    );
  }

  #[cfg(unix)]
  #[test]
  fn generate_invokes_backend_with_prompt_file() {
    use std::os::unix::fs::PermissionsExt;

    let temp_root = std::env::temp_dir().join(format!(
      "amentia-model-runtime-invocation-{}-{}",
      std::process::id(),
      prompt_file_nonce()
    ));
    std::fs::create_dir_all(&temp_root).expect("temp root");
    let binary_path = temp_root.join("llama-cli");
    let model_path = temp_root.join("model.gguf");
    let captured_prompt_path = temp_root.join("captured-prompt.txt");
    std::fs::write(&model_path, b"GGUFfake model").expect("model");
    std::fs::write(
      &binary_path,
      format!(
        "#!/bin/sh\n\
if [ \"$1\" = \"--help\" ]; then exit 0; fi\n\
prompt_file=\"\"\n\
while [ \"$#\" -gt 0 ]; do\n\
  if [ \"$1\" = \"-f\" ]; then\n\
    shift\n\
    prompt_file=\"$1\"\n\
  fi\n\
  shift\n\
done\n\
if [ -z \"$prompt_file\" ]; then echo \"missing prompt file\" >&2; exit 42; fi\n\
cat \"$prompt_file\" > \"{}\"\n\
printf 'fake model response\\n'\n",
        captured_prompt_path.display()
      ),
    )
    .expect("fake backend");
    std::fs::set_permissions(&binary_path, std::fs::Permissions::from_mode(0o755))
      .expect("permissions");

    let response = generate_with_llama_cpp(
      &binary_path,
      &model_path,
      &GenerateRequest {
        role: ModelRole::Summarizer,
        prompt: "secret prompt".to_string(),
        max_tokens: 16,
        timeout: Some(Duration::from_secs(5)),
        cancellation: None,
      },
      None,
    )
    .expect("generation");

    assert_eq!(response, "fake model response");
    assert_eq!(
      std::fs::read_to_string(&captured_prompt_path).expect("captured prompt"),
      "secret prompt"
    );

    let _ = std::fs::remove_dir_all(&temp_root);
  }
}
