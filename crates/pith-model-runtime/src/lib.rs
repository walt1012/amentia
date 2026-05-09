use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

mod discovery;
mod health;
mod inference;
mod types;
mod validation;

pub use inference::llama_cpp_timeout_seconds;
pub use types::*;

#[cfg(test)]
use discovery::discovery_roots;
use discovery::{
  normalize_path, resolve_binary_path, resolve_bootstrap_manifest, resolve_manifest,
  resolve_model_path, suggested_manifest_install_path, ManifestResolution,
};
use health::{display_path, missing_runtime_detail, model_metrics};
use inference::{generate_with_llama_cpp, generation_failure_text, request_is_cancelled};
#[cfg(test)]
use validation::sha256_hex;
use validation::validate_runtime_model_file;

#[derive(Debug, Clone)]
pub struct LocalModelRuntime {
  pack: ModelPackDescriptor,
  manifest: Option<ModelPackManifest>,
  source: String,
  backend: ModelBackend,
}

#[derive(Debug, Clone)]
enum ModelBackend {
  Unconfigured {
    detail: String,
    binary_path: Option<PathBuf>,
    model_path: Option<PathBuf>,
    manifest_path: Option<PathBuf>,
  },
  LlamaCppCli {
    binary_path: PathBuf,
    model_path: PathBuf,
    manifest_path: Option<PathBuf>,
  },
}

impl LocalModelRuntime {
  pub fn new_default() -> Self {
    let manifest_resolution = resolve_manifest();
    let binary_path = resolve_binary_path();
    let model_path = resolve_model_path(
      manifest_resolution
        .as_ref()
        .and_then(|resolution| resolution.manifest_path.parent().map(Path::to_path_buf)),
      manifest_resolution
        .as_ref()
        .map(|resolution| &resolution.manifest),
    );

    Self::from_resolution(manifest_resolution, binary_path, model_path)
  }

  pub fn from_paths(binary_path: Option<PathBuf>, model_path: Option<PathBuf>) -> Self {
    Self::from_resolution(None, binary_path, model_path)
  }

  fn from_resolution(
    manifest_resolution: Option<ManifestResolution>,
    binary_path: Option<PathBuf>,
    model_path: Option<PathBuf>,
  ) -> Self {
    let manifest = manifest_resolution
      .as_ref()
      .map(|resolution| resolution.manifest.clone());
    let pack = manifest
      .as_ref()
      .map(|manifest| ModelPackDescriptor {
        id: manifest.id.clone(),
        display_name: manifest.display_name.clone(),
        default_role: ModelRole::Default,
      })
      .unwrap_or_else(built_in_model_pack);
    let manifest_path = manifest_resolution
      .as_ref()
      .map(|resolution| resolution.manifest_path.clone());
    let source = manifest_resolution
      .as_ref()
      .map(|resolution| resolution.source.clone())
      .unwrap_or_else(|| "path-scan".to_string());

    match (binary_path, model_path) {
      (Some(binary_path), Some(model_path)) if binary_path.is_file() && model_path.is_file() => {
        match validate_runtime_model_file(&model_path, manifest.as_ref()) {
          Ok(()) => Self {
            pack,
            manifest,
            source,
            backend: ModelBackend::LlamaCppCli {
              binary_path,
              model_path,
              manifest_path,
            },
          },
          Err(error) => Self {
            pack,
            manifest,
            source,
            backend: ModelBackend::Unconfigured {
              detail: format!(
                "Local model runtime is unavailable because model integrity verification failed: {error}"
              ),
              binary_path: Some(binary_path),
              model_path: Some(model_path),
              manifest_path,
            },
          },
        }
      }
      (binary_path, model_path) => Self {
        pack,
        manifest,
        source,
        backend: ModelBackend::Unconfigured {
          detail: missing_runtime_detail(
            binary_path.as_deref(),
            model_path.as_deref(),
            manifest_path.as_deref(),
          ),
          binary_path,
          model_path,
          manifest_path,
        },
      },
    }
  }

  pub fn health(&self) -> ModelHealth {
    match &self.backend {
      ModelBackend::Unconfigured {
        detail,
        binary_path,
        model_path,
        manifest_path,
      } => {
        let metrics = model_metrics(
          self.manifest.as_ref(),
          binary_path.as_deref(),
          model_path.as_deref(),
          manifest_path.as_deref(),
          false,
        );
        ModelHealth {
          pack_id: self.pack.id.clone(),
          display_name: self.pack.display_name.clone(),
          backend: "unconfigured".to_string(),
          status: "unavailable".to_string(),
          detail: detail.clone(),
          source: self.source.clone(),
          binary_path: binary_path.as_ref().map(display_path),
          model_path: model_path.as_ref().map(display_path),
          manifest_path: manifest_path.as_ref().map(display_path),
          metrics,
        }
      }
      ModelBackend::LlamaCppCli {
        binary_path,
        model_path,
        manifest_path,
      } => {
        let metrics = model_metrics(
          self.manifest.as_ref(),
          Some(binary_path.as_path()),
          Some(model_path.as_path()),
          manifest_path.as_deref(),
          true,
        );
        ModelHealth {
          pack_id: self.pack.id.clone(),
          display_name: self.pack.display_name.clone(),
          backend: "llama.cpp".to_string(),
          status: "ready".to_string(),
          detail: "Local llama.cpp CLI inference is configured.".to_string(),
          source: self.source.clone(),
          binary_path: Some(display_path(binary_path)),
          model_path: Some(display_path(model_path)),
          manifest_path: manifest_path.as_ref().map(display_path),
          metrics,
        }
      }
    }
  }

  pub fn generate(&self, request: GenerateRequest) -> GenerateResponse {
    if request_is_cancelled(&request) {
      return self.generate_cancelled(&request.role);
    }

    match &self.backend {
      ModelBackend::Unconfigured { detail, .. } => {
        self.generate_failure("unconfigured", "unavailable", &request.role, detail)
      }
      ModelBackend::LlamaCppCli {
        binary_path,
        model_path,
        ..
      } => match generate_with_llama_cpp(binary_path, model_path, &request, self.manifest.as_ref())
      {
        Ok(text) => GenerateResponse {
          text,
          backend: "llama.cpp".to_string(),
          status: "ready".to_string(),
          model_id: self.pack.id.clone(),
        },
        Err(error) => {
          if request_is_cancelled(&request) {
            self.generate_cancelled(&request.role)
          } else {
            self.generate_failure(
              "llama.cpp",
              "error",
              &request.role,
              &format!("Local llama.cpp inference failed: {error}"),
            )
          }
        }
      },
    }
  }

  fn generate_cancelled(&self, role: &ModelRole) -> GenerateResponse {
    GenerateResponse {
      text: generation_failure_text(
        role,
        "local model generation was cancelled before completion.",
      ),
      backend: "local".to_string(),
      status: "cancelled".to_string(),
      model_id: self.pack.id.clone(),
    }
  }

  fn generate_failure(
    &self,
    backend: &str,
    status: &str,
    role: &ModelRole,
    detail: &str,
  ) -> GenerateResponse {
    GenerateResponse {
      text: generation_failure_text(role, detail),
      backend: backend.to_string(),
      status: status.to_string(),
      model_id: self.pack.id.clone(),
    }
  }

  pub fn bootstrap_pack_metadata(&self) -> Result<ModelBootstrap> {
    let target_manifest_path = suggested_manifest_install_path();
    let resolution = resolve_bootstrap_manifest(&target_manifest_path)
      .context("failed to locate a default model-pack.json for bootstrap")?;
    let target_directory = target_manifest_path
      .parent()
      .context("suggested manifest path has no parent directory")?;
    fs::create_dir_all(target_directory)
      .with_context(|| format!("failed to create {}", target_directory.display()))?;

    let mut copied_files = vec![];
    if normalize_path(&resolution.manifest_path) != normalize_path(&target_manifest_path) {
      fs::copy(&resolution.manifest_path, &target_manifest_path).with_context(|| {
        format!(
          "failed to copy {} to {}",
          resolution.manifest_path.display(),
          target_manifest_path.display()
        )
      })?;
      copied_files.push(target_manifest_path.clone());
    }

    let source_readme_path = resolution
      .manifest_path
      .parent()
      .map(|directory| directory.join("README.md"))
      .filter(|path| path.is_file());
    let target_readme_path = target_directory.join("README.md");
    let readme_path = if let Some(source_readme_path) = source_readme_path {
      if normalize_path(&source_readme_path) != normalize_path(&target_readme_path) {
        fs::copy(&source_readme_path, &target_readme_path).with_context(|| {
          format!(
            "failed to copy {} to {}",
            source_readme_path.display(),
            target_readme_path.display()
          )
        })?;
        copied_files.push(target_readme_path.clone());
      }
      Some(target_readme_path)
    } else {
      None
    };

    Ok(ModelBootstrap {
      manifest_path: target_manifest_path,
      readme_path,
      copied_files,
    })
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::env;
  use std::io::ErrorKind;
  use std::path::PathBuf;
  use std::sync::{Mutex, MutexGuard};
  use std::time::{SystemTime, UNIX_EPOCH};

  static ENVIRONMENT_LOCK: Mutex<()> = Mutex::new(());

  #[test]
  fn runtime_reports_unconfigured_backend_when_paths_are_missing() {
    let runtime = LocalModelRuntime::from_paths(None, None);
    let health = runtime.health();

    assert_eq!(health.display_name, "LFM2.5-350M");
    assert_eq!(health.backend, "unconfigured");
    assert_eq!(health.status, "unavailable");
    assert!(health.detail.contains("Local model runtime is unavailable"));
    assert!(!health.detail.to_lowercase().contains("degraded generation"));
    assert!(health.metrics.contains_key("contextSize"));
    assert_eq!(health.metrics["readiness"], "unconfigured");
    assert_eq!(health.metrics["packReady"], "false");
    assert!(health.metrics["installHint"].contains("model-pack.json"));
    assert!(health.metrics.contains_key("suggestedManifestPath"));
    assert!(health.metrics.contains_key("suggestedModelPath"));
    assert!(health.metrics.contains_key("suggestedBinaryPath"));
  }

  #[test]
  fn unconfigured_generation_returns_unavailable_error() {
    let runtime = LocalModelRuntime::from_paths(None, None);
    let response = runtime.generate(GenerateRequest {
      role: ModelRole::Summarizer,
      prompt: "Summarize this test prompt.".to_string(),
      max_tokens: 96,
      cancellation: None,
    });

    assert!(response
      .text
      .contains("could not produce a local summarizer response"));
    assert_eq!(response.backend, "unconfigured");
    assert_eq!(response.status, "unavailable");
    assert_eq!(response.model_id, "lfm2.5-350m");
  }

  #[test]
  fn runtime_health_uses_selected_manifest_identity() {
    let runtime = LocalModelRuntime::from_resolution(
      Some(ManifestResolution {
        manifest: ModelPackManifest {
          id: "granite-4.0-h-350m".to_string(),
          display_name: "Granite 4.0-H-350M Q4_K_M".to_string(),
          file_name: "granite-4.0-h-350m-Q4_K_M.gguf".to_string(),
          context_size: 4096,
          model_context_size: Some(32_768),
          max_output_tokens: 192,
          backend: "llama.cpp".to_string(),
          license: Some("apache-2.0".to_string()),
          homepage: None,
          download_url: None,
          sha256: None,
          size_bytes: None,
        },
        manifest_path: PathBuf::from("model-pack.json"),
        source: "environment".to_string(),
      }),
      None,
      None,
    );
    let health = runtime.health();

    assert_eq!(health.pack_id, "granite-4.0-h-350m");
    assert_eq!(health.display_name, "Granite 4.0-H-350M Q4_K_M");
    assert_eq!(health.metrics["contextSize"], "4096");
    assert_eq!(health.metrics["modelContextSize"], "32768");
    assert_eq!(health.metrics["maxOutputTokens"], "192");
  }

  #[test]
  fn runtime_accepts_manifest_verified_model_file() {
    let temp_root = unique_temp_directory("model-verified");
    fs::create_dir_all(&temp_root).expect("temp root");
    let binary_path = temp_root.join("llama-cli");
    let model_path = temp_root.join("test-model.gguf");
    let manifest_path = temp_root.join("model-pack.json");
    fs::write(&binary_path, "fake binary").expect("binary");
    fs::write(&model_path, b"GGUFmodel bytes").expect("model");
    let manifest = test_manifest(
      "test-model.gguf",
      Some(sha256_hex(&model_path).expect("model sha256")),
      Some(fs::metadata(&model_path).expect("model metadata").len()),
    );

    let runtime = LocalModelRuntime::from_resolution(
      Some(ManifestResolution {
        manifest,
        manifest_path,
        source: "test".to_string(),
      }),
      Some(binary_path),
      Some(model_path),
    );
    let health = runtime.health();

    assert_eq!(health.backend, "llama.cpp");
    assert_eq!(health.status, "ready");
    assert_eq!(health.metrics["readiness"], "ready");

    remove_temp_directory(&temp_root);
  }

  #[test]
  fn runtime_rejects_model_with_wrong_manifest_checksum() {
    let temp_root = unique_temp_directory("model-bad-checksum");
    fs::create_dir_all(&temp_root).expect("temp root");
    let binary_path = temp_root.join("llama-cli");
    let model_path = temp_root.join("test-model.gguf");
    let manifest_path = temp_root.join("model-pack.json");
    fs::write(&binary_path, "fake binary").expect("binary");
    fs::write(&model_path, b"GGUFmodel bytes").expect("model");

    let runtime = LocalModelRuntime::from_resolution(
      Some(ManifestResolution {
        manifest: test_manifest("test-model.gguf", Some("0".repeat(64)), Some(15)),
        manifest_path,
        source: "test".to_string(),
      }),
      Some(binary_path),
      Some(model_path),
    );
    let health = runtime.health();

    assert_eq!(health.backend, "unconfigured");
    assert_eq!(health.status, "unavailable");
    assert!(health.detail.contains("checksum mismatch"));
    assert_eq!(health.metrics["readiness"], "misconfigured");

    remove_temp_directory(&temp_root);
  }

  #[test]
  fn discovery_roots_include_configured_model_directories() {
    let _environment = lock_environment();
    let previous_model_pack_root = env::var("PITH_MODEL_PACK_ROOT").ok();
    let previous_data_dir = env::var("PITH_DATA_DIR").ok();

    env::set_var("PITH_MODEL_PACK_ROOT", "C:/tmp/pith-pack-root");
    env::set_var("PITH_DATA_DIR", "C:/tmp/pith-data");

    let roots = discovery_roots();

    match previous_model_pack_root {
      Some(value) => env::set_var("PITH_MODEL_PACK_ROOT", value),
      None => env::remove_var("PITH_MODEL_PACK_ROOT"),
    }
    match previous_data_dir {
      Some(value) => env::set_var("PITH_DATA_DIR", value),
      None => env::remove_var("PITH_DATA_DIR"),
    }

    assert!(roots.contains(&PathBuf::from("C:/tmp/pith-pack-root")));
    assert!(roots.contains(&PathBuf::from("C:/tmp/pith-data")));
    assert!(roots.contains(&PathBuf::from("C:/tmp/pith-data").join("models")));
  }

  #[test]
  fn bootstrap_pack_metadata_copies_manifest_and_readme_into_data_dir() {
    let _environment = lock_environment();
    let temp_root = unique_temp_directory("model-bootstrap");
    let source_root = temp_root.join("source");
    let source_pack_root = source_root
      .join("models")
      .join("builtin")
      .join("lfm2.5-350m");
    fs::create_dir_all(&source_pack_root).expect("source pack root");
    fs::write(
      source_pack_root.join("model-pack.json"),
      r#"{
  "id": "lfm2.5-350m",
  "display_name": "LFM2.5-350M Q4_K_M",
  "file_name": "LFM2.5-350M-Q4_K_M.gguf",
  "context_size": 4096,
  "model_context_size": 32768,
  "max_output_tokens": 160,
  "backend": "llama.cpp",
  "license": "lfm1.0",
  "download_url": "https://huggingface.co/LiquidAI/LFM2.5-350M-GGUF/resolve/main/LFM2.5-350M-Q4_K_M.gguf",
  "sha256": "7e6f72643caafc9a68256686638c4d7916f2cec76d1df478d4c3ddcd95a6aed4",
  "size_bytes": 229312224
}"#,
    )
    .expect("manifest");
    fs::write(
      source_pack_root.join("README.md"),
      "Default model pack metadata",
    )
    .expect("readme");

    let data_root = temp_root.join("data");
    let previous_pack_root = env::var("PITH_MODEL_PACK_ROOT").ok();
    let previous_data_dir = env::var("PITH_DATA_DIR").ok();
    let previous_manifest = env::var("PITH_MODEL_PACK_MANIFEST").ok();

    env::set_var("PITH_MODEL_PACK_ROOT", &source_root);
    env::set_var("PITH_DATA_DIR", &data_root);
    env::remove_var("PITH_MODEL_PACK_MANIFEST");

    let runtime = LocalModelRuntime::new_default();
    let bootstrap = runtime
      .bootstrap_pack_metadata()
      .expect("bootstrap metadata");

    restore_env_var("PITH_MODEL_PACK_ROOT", previous_pack_root);
    restore_env_var("PITH_DATA_DIR", previous_data_dir);
    restore_env_var("PITH_MODEL_PACK_MANIFEST", previous_manifest);

    assert!(bootstrap.manifest_path.is_file());
    assert_eq!(
      fs::read_to_string(&bootstrap.manifest_path).expect("copied manifest"),
      fs::read_to_string(source_pack_root.join("model-pack.json")).expect("source manifest")
    );
    let copied_readme_path = bootstrap.readme_path.expect("copied readme");
    assert!(copied_readme_path.is_file());
    assert_eq!(bootstrap.copied_files.len(), 2);

    remove_temp_directory(&temp_root);
  }

  fn test_manifest(
    file_name: &str,
    sha256: Option<String>,
    size_bytes: Option<u64>,
  ) -> ModelPackManifest {
    ModelPackManifest {
      id: "test-model".to_string(),
      display_name: "Test Model".to_string(),
      file_name: file_name.to_string(),
      context_size: 4096,
      model_context_size: Some(4096),
      max_output_tokens: 128,
      backend: "llama.cpp".to_string(),
      license: Some("apache-2.0".to_string()),
      homepage: None,
      download_url: None,
      sha256,
      size_bytes,
    }
  }

  fn restore_env_var(key: &str, value: Option<String>) {
    match value {
      Some(value) => env::set_var(key, value),
      None => env::remove_var(key),
    }
  }

  fn lock_environment() -> MutexGuard<'static, ()> {
    ENVIRONMENT_LOCK
      .lock()
      .unwrap_or_else(|poisoned| poisoned.into_inner())
  }

  fn unique_temp_directory(prefix: &str) -> PathBuf {
    let nanos = SystemTime::now()
      .duration_since(UNIX_EPOCH)
      .expect("system time")
      .as_nanos();
    env::temp_dir().join(format!("pith-{prefix}-{nanos}"))
  }

  fn remove_temp_directory(path: &Path) {
    if let Err(error) = fs::remove_dir_all(path) {
      if error.kind() != ErrorKind::NotFound {
        panic!("failed to remove {}: {error}", path.display());
      }
    }
  }
}
