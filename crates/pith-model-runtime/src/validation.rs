use std::fs;
use std::io::Read;
use std::path::Path;

use anyhow::{bail, Context, Result};
use sha2::{Digest, Sha256};

use crate::ModelPackManifest;

const GGUF_MAGIC: [u8; 4] = *b"GGUF";

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
