use std::fs;
use std::io::Read;
use std::path::Path;
use std::process::{Command, Output, Stdio};
use std::time::Duration;

use anyhow::{bail, Context, Result};
use pith_process::{
  configure_process_group, join_bounded_pipe_reader, read_bounded_pipe_in_background,
  wait_for_child, ChildExitReason,
};
use sha2::{Digest, Sha256};

use crate::ModelPackManifest;

const GGUF_MAGIC: [u8; 4] = *b"GGUF";
const BACKEND_PROBE_TIMEOUT: Duration = Duration::from_secs(10);
const BACKEND_PROBE_POLL_INTERVAL: Duration = Duration::from_millis(50);
const BACKEND_PROBE_OUTPUT_LIMIT: usize = 64 * 1024;

pub(crate) fn validate_runtime_backend(binary_path: &Path) -> Result<()> {
  let metadata = fs::metadata(binary_path)
    .with_context(|| format!("failed to inspect backend {}", binary_path.display()))?;
  if !metadata.is_file() {
    bail!("backend path is not a file: {}", binary_path.display());
  }

  let output = run_backend_probe_with_timeout(binary_path)
    .with_context(|| format!("failed to launch backend {}", binary_path.display()))?;
  if !output.status.success() {
    let detail = String::from_utf8_lossy(&output.stderr).trim().to_string();
    bail!(
      "backend launch probe failed with status {}: {}",
      output.status,
      if detail.is_empty() {
        "no stderr output".to_string()
      } else {
        detail
      }
    );
  }

  Ok(())
}

pub(crate) fn validate_runtime_model_file(
  model_path: &Path,
  manifest: Option<&ModelPackManifest>,
) -> Result<()> {
  let Some(manifest) = manifest else {
    bail!("model pack manifest is required for local model verification");
  };

  let actual_file_name = model_path
    .file_name()
    .and_then(|name| name.to_str())
    .unwrap_or_default();
  if actual_file_name != manifest.file_name {
    bail!(
      "model file name mismatch: expected {}, found {}",
      manifest.file_name,
      actual_file_name
    );
  }

  let metadata = fs::metadata(model_path)
    .with_context(|| format!("failed to inspect model file {}", model_path.display()))?;
  if let Some(expected_size_bytes) = manifest.size_bytes {
    if metadata.len() != expected_size_bytes {
      bail!(
        "model size mismatch: expected {} bytes, found {} bytes",
        expected_size_bytes,
        metadata.len()
      );
    }
  }

  validate_gguf_magic(model_path)?;

  let expected_sha256 = manifest
    .sha256
    .as_deref()
    .map(str::trim)
    .filter(|value| !value.is_empty())
    .context("model manifest is missing SHA-256 metadata")?;
  let actual_sha256 = sha256_hex(model_path)?;
  if !expected_sha256.eq_ignore_ascii_case(&actual_sha256) {
    bail!(
      "model checksum mismatch: expected {}, found {}",
      expected_sha256,
      actual_sha256
    );
  }

  Ok(())
}

fn validate_gguf_magic(model_path: &Path) -> Result<()> {
  let mut file = fs::File::open(model_path)
    .with_context(|| format!("failed to open model file {}", model_path.display()))?;
  let mut magic = [0_u8; 4];
  file
    .read_exact(&mut magic)
    .with_context(|| format!("failed to read GGUF header from {}", model_path.display()))?;
  if magic != GGUF_MAGIC {
    bail!("model file is not a GGUF file");
  }

  Ok(())
}

fn run_backend_probe_with_timeout(binary_path: &Path) -> Result<Output> {
  let mut child = build_backend_probe_command(binary_path).spawn()?;
  let stdout_reader = child
    .stdout
    .take()
    .map(|reader| read_bounded_pipe_in_background(reader, BACKEND_PROBE_OUTPUT_LIMIT));
  let stderr_reader = child
    .stderr
    .take()
    .map(|reader| read_bounded_pipe_in_background(reader, BACKEND_PROBE_OUTPUT_LIMIT));
  let wait = wait_for_child(
    &mut child,
    BACKEND_PROBE_TIMEOUT,
    BACKEND_PROBE_POLL_INTERVAL,
    Duration::from_millis(200),
    || false,
  )?;

  let stdout = join_bounded_pipe_reader(stdout_reader).bytes;
  let stderr = join_bounded_pipe_reader(stderr_reader).bytes;
  if wait.reason == ChildExitReason::TimedOut {
    let detail = String::from_utf8_lossy(&stderr).trim().to_string();
    if detail.is_empty() {
      bail!(
        "backend launch probe timed out after {} seconds",
        BACKEND_PROBE_TIMEOUT.as_secs()
      );
    }

    bail!(
      "backend launch probe timed out after {} seconds: {}",
      BACKEND_PROBE_TIMEOUT.as_secs(),
      detail
    );
  }

  Ok(Output {
    status: wait.status,
    stdout,
    stderr,
  })
}

fn build_backend_probe_command(binary_path: &Path) -> Command {
  let mut process = Command::new(binary_path);
  configure_process_group(&mut process);
  process
    .arg("--help")
    .stdin(Stdio::null())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped());
  process
}

pub(crate) fn sha256_hex(path: &Path) -> Result<String> {
  let mut file =
    fs::File::open(path).with_context(|| format!("failed to open {}", path.display()))?;
  let mut hasher = Sha256::new();
  let mut buffer = [0_u8; 1024 * 1024];

  loop {
    let bytes_read = file
      .read(&mut buffer)
      .with_context(|| format!("failed to read {}", path.display()))?;
    if bytes_read == 0 {
      break;
    }
    hasher.update(&buffer[..bytes_read]);
  }

  Ok(format!("{:x}", hasher.finalize()))
}
