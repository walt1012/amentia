use super::*;
use crate::discovery::discovery_roots;
use crate::validation::sha256_hex;
use std::env;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
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
    timeout: None,
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
  let binary_path = env::current_exe().expect("current test executable");
  let model_path = temp_root.join("test-model.gguf");
  let manifest_path = temp_root.join("model-pack.json");
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
  assert_eq!(health.metrics["invocationMode"], "bounded-llama-cpp");
  assert_eq!(health.metrics["promptInput"], "temporary-file");

  remove_temp_directory(&temp_root);
}

#[test]
fn runtime_rejects_backend_that_cannot_launch() {
  let temp_root = unique_temp_directory("backend-launch-failure");
  fs::create_dir_all(&temp_root).expect("temp root");
  let binary_path = temp_root.join("llama-cli");
  let model_path = temp_root.join("test-model.gguf");
  let manifest_path = temp_root.join("model-pack.json");
  fs::write(&binary_path, "not an executable backend").expect("binary");
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

  assert_eq!(health.backend, "unconfigured");
  assert_eq!(health.status, "unavailable");
  assert!(health
    .detail
    .contains("local inference setup verification failed"));
  assert!(health.detail.contains("backend"));
  assert_eq!(health.metrics["readiness"], "misconfigured");

  remove_temp_directory(&temp_root);
}

#[test]
fn runtime_rejects_model_with_wrong_manifest_checksum() {
  let temp_root = unique_temp_directory("model-bad-checksum");
  fs::create_dir_all(&temp_root).expect("temp root");
  let binary_path = env::current_exe().expect("current test executable");
  let model_path = temp_root.join("test-model.gguf");
  let manifest_path = temp_root.join("model-pack.json");
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
