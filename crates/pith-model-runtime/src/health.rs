use std::collections::HashMap;
use std::path::Path;

use crate::discovery::{
  suggested_binary_install_path, suggested_manifest_install_path, suggested_model_install_path,
};
use crate::ModelPackManifest;

pub(crate) fn model_metrics(
  manifest: Option<&ModelPackManifest>,
  binary_path: Option<&Path>,
  model_path: Option<&Path>,
  manifest_path: Option<&Path>,
  is_runtime_ready: bool,
) -> HashMap<String, String> {
  let mut metrics = HashMap::new();
  if let Some(manifest) = manifest {
    metrics.insert("backend".to_string(), manifest.backend.clone());
    metrics.insert("contextSize".to_string(), manifest.context_size.to_string());
    if let Some(model_context_size) = manifest.model_context_size {
      metrics.insert(
        "modelContextSize".to_string(),
        model_context_size.to_string(),
      );
    }
    metrics.insert(
      "maxOutputTokens".to_string(),
      manifest.max_output_tokens.to_string(),
    );
    metrics.insert("fileName".to_string(), manifest.file_name.clone());
    if let Some(license) = &manifest.license {
      metrics.insert("license".to_string(), license.clone());
    }
    if let Some(homepage) = &manifest.homepage {
      metrics.insert("homepage".to_string(), homepage.clone());
    }
    if let Some(download_url) = &manifest.download_url {
      metrics.insert("downloadUrl".to_string(), download_url.clone());
    }
    if let Some(sha256) = &manifest.sha256 {
      metrics.insert("sha256".to_string(), sha256.clone());
    }
    if let Some(size_bytes) = manifest.size_bytes {
      metrics.insert("sizeBytes".to_string(), size_bytes.to_string());
    }
  } else {
    metrics.insert("backend".to_string(), "llama.cpp".to_string());
    metrics.insert("contextSize".to_string(), "4096".to_string());
    metrics.insert("maxOutputTokens".to_string(), "160".to_string());
  }

  let suggested_file_name = manifest
    .map(|item| item.file_name.as_str())
    .unwrap_or("LFM2.5-350M-Q4_K_M.gguf");
  let suggested_manifest_path = suggested_manifest_install_path();
  let suggested_model_path = manifest_path
    .and_then(|path| path.parent())
    .map(|directory| directory.join(suggested_file_name))
    .unwrap_or_else(|| suggested_model_install_path(suggested_file_name));
  let suggested_binary_path = suggested_binary_install_path();
  metrics.insert(
    "suggestedManifestPath".to_string(),
    suggested_manifest_path.display().to_string(),
  );
  metrics.insert(
    "suggestedModelPath".to_string(),
    suggested_model_path.display().to_string(),
  );
  metrics.insert(
    "suggestedBinaryPath".to_string(),
    suggested_binary_path.display().to_string(),
  );
  let readiness = model_readiness(binary_path, model_path, manifest_path, is_runtime_ready);
  metrics.insert("readiness".to_string(), readiness.to_string());
  metrics.insert(
    "packReady".to_string(),
    if is_runtime_ready { "true" } else { "false" }.to_string(),
  );
  if is_runtime_ready {
    metrics.insert("invocationMode".to_string(), "bounded-llama-cpp".to_string());
    metrics.insert("promptInput".to_string(), "temporary-file".to_string());
  }
  metrics.insert(
    "binaryPresent".to_string(),
    binary_path.is_some().to_string(),
  );
  metrics.insert("modelPresent".to_string(), model_path.is_some().to_string());
  metrics.insert(
    "manifestPresent".to_string(),
    manifest_path.is_some().to_string(),
  );
  metrics.insert(
    "installHint".to_string(),
    install_hint(manifest, readiness, binary_path, model_path, manifest_path),
  );

  metrics
}

fn model_readiness(
  binary_path: Option<&Path>,
  model_path: Option<&Path>,
  manifest_path: Option<&Path>,
  is_runtime_ready: bool,
) -> &'static str {
  if is_runtime_ready {
    return "ready";
  }

  match (binary_path, model_path, manifest_path) {
    (Some(_), None, Some(_)) | (Some(_), None, None) => "model_missing",
    (None, Some(_), Some(_)) | (None, Some(_), None) => "binary_missing",
    (None, None, Some(_)) => "manifest_only",
    (Some(_), Some(_), Some(_)) | (Some(_), Some(_), None) => "misconfigured",
    (None, None, None) => "unconfigured",
  }
}

fn install_hint(
  manifest: Option<&ModelPackManifest>,
  readiness: &str,
  binary_path: Option<&Path>,
  model_path: Option<&Path>,
  manifest_path: Option<&Path>,
) -> String {
  let file_name = manifest
    .map(|item| item.file_name.as_str())
    .unwrap_or("LFM2.5-350M-Q4_K_M.gguf");
  let suggested_manifest = suggested_manifest_install_path();
  let suggested_model = manifest_path
    .and_then(|path| path.parent())
    .map(|directory| directory.join(file_name))
    .unwrap_or_else(|| suggested_model_install_path(file_name));
  let suggested_binary = suggested_binary_install_path();

  match readiness {
    "ready" => format!(
      "Local inference is ready. Keep the manifest near {} and use {} if you need to override discovery.",
      file_name,
      "PITH_MODEL_PATH"
    ),
    "model_missing" => format!(
      "Place {} at {} or set PITH_MODEL_PATH. Current binary candidate: {}.",
      file_name,
      suggested_model.display(),
      binary_path
        .map(display_path)
        .unwrap_or_else(|| suggested_binary.display().to_string())
    ),
    "binary_missing" => format!(
      "Repair the packaged local inference backend or reinstall Pith. Expected llama.cpp backend: {}. Current model candidate: {}.",
      suggested_binary.display(),
      model_path
        .map(display_path)
        .unwrap_or_else(|| suggested_model.display().to_string())
    ),
    "manifest_only" => format!(
      "Keep the manifest at {} and download {} in-app. If the backend is missing, repair or reinstall Pith. Expected backend: {}.",
      manifest_path
        .map(display_path)
        .unwrap_or_else(|| suggested_manifest.display().to_string()),
      file_name,
      suggested_binary.display()
    ),
    "misconfigured" => format!(
      "Resolved candidates exist but local inference is not ready. Verify the manifest at {}, model at {}, and packaged backend at {}.",
      manifest_path
        .map(display_path)
        .unwrap_or_else(|| suggested_manifest.display().to_string()),
      model_path
        .map(display_path)
        .unwrap_or_else(|| suggested_model.display().to_string()),
      binary_path
        .map(display_path)
        .unwrap_or_else(|| suggested_binary.display().to_string())
    ),
    _ => format!(
      "Use the in-app model manager to download {}. If the backend is missing, repair or reinstall Pith. Expected manifest: {}. Expected backend: {}.",
      file_name,
      suggested_manifest.display(),
      suggested_binary.display()
    ),
  }
}

pub(crate) fn missing_runtime_detail(
  binary_path: Option<&Path>,
  model_path: Option<&Path>,
  manifest_path: Option<&Path>,
) -> String {
  match (binary_path, model_path, manifest_path) {
    (Some(binary_path), Some(model_path), Some(manifest_path)) => format!(
      "Local model runtime is unavailable because {} or {} is missing. Manifest: {}.",
      binary_path.display(),
      model_path.display(),
      manifest_path.display()
    ),
    (Some(binary_path), Some(model_path), None) => format!(
      "Local model runtime is unavailable because {} or {} is missing.",
      binary_path.display(),
      model_path.display()
    ),
    (Some(binary_path), None, Some(manifest_path)) => format!(
      "Local model runtime is unavailable because the model file is not configured or missing. Binary candidate: {}. Manifest: {}.",
      binary_path.display(),
      manifest_path.display()
    ),
    (Some(binary_path), None, None) => format!(
      "Local model runtime is unavailable because the model file is not configured or missing. Binary candidate: {}.",
      binary_path.display()
    ),
    (None, Some(model_path), Some(manifest_path)) => format!(
      "Local model runtime is unavailable because the packaged llama.cpp backend is missing. Model candidate: {}. Manifest: {}.",
      model_path.display(),
      manifest_path.display()
    ),
    (None, Some(model_path), None) => format!(
      "Local model runtime is unavailable because the packaged llama.cpp backend is missing. Model candidate: {}.",
      model_path.display()
    ),
    (None, None, Some(manifest_path)) => format!(
      "Local model runtime is unavailable because no packaged llama.cpp backend or resolved local model file is configured yet. Manifest: {}.",
      manifest_path.display()
    ),
    (None, None, None) => "Local model runtime is unavailable because no packaged llama.cpp backend or local model pack is configured yet.".to_string(),
  }
}

pub(crate) fn display_path(path: impl AsRef<Path>) -> String {
  path.as_ref().display().to_string()
}
