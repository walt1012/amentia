use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModelRole {
  Default,
  Planner,
  Coder,
  Summarizer,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPackDescriptor {
  pub id: String,
  pub display_name: String,
  pub default_role: ModelRole,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelHealth {
  pub pack_id: String,
  pub display_name: String,
  pub backend: String,
  pub status: String,
  pub detail: String,
  pub source: String,
  pub binary_path: Option<String>,
  pub model_path: Option<String>,
  pub manifest_path: Option<String>,
  pub metrics: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPackManifest {
  pub id: String,
  pub display_name: String,
  pub file_name: String,
  pub context_size: usize,
  pub max_output_tokens: usize,
  pub backend: String,
}

#[derive(Debug, Clone)]
pub struct GenerateRequest {
  pub role: ModelRole,
  pub prompt: String,
  pub fallback: String,
  pub max_tokens: usize,
}

#[derive(Debug, Clone)]
pub struct GenerateResponse {
  pub text: String,
  pub backend: String,
  pub status: String,
  pub model_id: String,
}

#[derive(Debug, Clone)]
pub struct LocalModelRuntime {
  pack: ModelPackDescriptor,
  manifest: Option<ModelPackManifest>,
  source: String,
  backend: ModelBackend,
}

#[derive(Debug, Clone)]
enum ModelBackend {
  Heuristic {
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
    let pack = built_in_model_pack();
    let manifest = manifest_resolution
      .as_ref()
      .map(|resolution| resolution.manifest.clone());
    let manifest_path = manifest_resolution
      .as_ref()
      .map(|resolution| resolution.manifest_path.clone());
    let source = manifest_resolution
      .as_ref()
      .map(|resolution| resolution.source.clone())
      .unwrap_or_else(|| "path-scan".to_string());

    match (binary_path, model_path) {
      (Some(binary_path), Some(model_path)) if binary_path.is_file() && model_path.is_file() => {
        Self {
          pack,
          manifest,
          source,
          backend: ModelBackend::LlamaCppCli {
            binary_path,
            model_path,
            manifest_path,
          },
        }
      }
      (binary_path, model_path) => Self {
        pack,
        manifest,
        source,
        backend: ModelBackend::Heuristic {
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
      ModelBackend::Heuristic {
        detail,
        binary_path,
        model_path,
        manifest_path,
      } => ModelHealth {
        pack_id: self.pack.id.clone(),
        display_name: self.pack.display_name.clone(),
        backend: "heuristic".to_string(),
        status: "fallback".to_string(),
        detail: detail.clone(),
        source: self.source.clone(),
        binary_path: binary_path.as_ref().map(display_path),
        model_path: model_path.as_ref().map(display_path),
        manifest_path: manifest_path.as_ref().map(display_path),
        metrics: model_metrics(self.manifest.as_ref()),
      },
      ModelBackend::LlamaCppCli {
        binary_path,
        model_path,
        manifest_path,
      } => ModelHealth {
        pack_id: self.pack.id.clone(),
        display_name: self.pack.display_name.clone(),
        backend: "llama.cpp".to_string(),
        status: "ready".to_string(),
        detail: "Local llama.cpp CLI inference is configured.".to_string(),
        source: self.source.clone(),
        binary_path: Some(display_path(binary_path)),
        model_path: Some(display_path(model_path)),
        manifest_path: manifest_path.as_ref().map(display_path),
        metrics: model_metrics(self.manifest.as_ref()),
      },
    }
  }

  pub fn generate(&self, request: GenerateRequest) -> GenerateResponse {
    match &self.backend {
      ModelBackend::Heuristic { .. } => self.generate_fallback(request, "fallback".to_string()),
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
        Err(_) => self.generate_fallback(request, "fallback".to_string()),
      },
    }
  }

  fn generate_fallback(&self, request: GenerateRequest, status: String) -> GenerateResponse {
    let mut text = request.fallback.trim().to_string();
    if text.is_empty() {
      text = fallback_from_prompt(&request.prompt, &request.role);
    }

    GenerateResponse {
      text,
      backend: "heuristic".to_string(),
      status,
      model_id: self.pack.id.clone(),
    }
  }
}

pub fn built_in_model_pack() -> ModelPackDescriptor {
  ModelPackDescriptor {
    id: "lfm2.5-350m".to_string(),
    display_name: "LFM2.5-350M".to_string(),
    default_role: ModelRole::Default,
  }
}

#[derive(Debug, Clone)]
struct ManifestResolution {
  manifest: ModelPackManifest,
  manifest_path: PathBuf,
  source: String,
}

fn resolve_binary_path() -> Option<PathBuf> {
  if let Ok(path) = env::var("CAVELL_LLAMACPP_PATH") {
    return Some(PathBuf::from(path));
  }

  default_binary_candidates()
    .into_iter()
    .find(|path| path.is_file())
}

fn resolve_model_path(
  manifest_directory: Option<PathBuf>,
  manifest: Option<&ModelPackManifest>,
) -> Option<PathBuf> {
  if let Ok(path) = env::var("CAVELL_LFM_MODEL_PATH") {
    return Some(PathBuf::from(path));
  }

  if let (Some(manifest_directory), Some(manifest)) = (manifest_directory, manifest) {
    let manifest_candidate = manifest_directory.join(&manifest.file_name);
    if manifest_candidate.is_file() {
      return Some(manifest_candidate);
    }
  }

  default_model_candidates()
    .into_iter()
    .find(|path| path.is_file())
}

fn resolve_manifest() -> Option<ManifestResolution> {
  let env_manifest = env::var("CAVELL_MODEL_PACK_MANIFEST")
    .ok()
    .map(PathBuf::from);
  let mut candidates = vec![];

  if let Some(env_manifest) = env_manifest {
    candidates.push((env_manifest, "environment".to_string()));
  }

  for current_dir in discovery_roots() {
    candidates.push((
      current_dir
        .join("models")
        .join("builtin")
        .join("lfm2.5-350m")
        .join("model-pack.json"),
      "bundle-manifest".to_string(),
    ));
    candidates.push((
      current_dir
        .join("model-packs")
        .join("lfm2.5-350m")
        .join("model-pack.json"),
      "bundle-manifest".to_string(),
    ));
  }

  for (path, source) in candidates {
    if !path.is_file() {
      continue;
    }

    if let Ok(manifest) = read_manifest(&path) {
      return Some(ManifestResolution {
        manifest,
        manifest_path: path,
        source,
      });
    }
  }

  None
}

fn default_binary_candidates() -> Vec<PathBuf> {
  let mut candidates = vec![];
  let binary_names = if cfg!(windows) {
    vec!["llama-cli.exe", "main.exe"]
  } else {
    vec!["llama-cli", "main"]
  };

  for current_dir in discovery_roots() {
    for name in &binary_names {
      candidates.push(current_dir.join("third_party").join("llama.cpp").join(name));
      candidates.push(current_dir.join("tools").join("llama.cpp").join(name));
    }
  }

  if let Ok(home_dir) = env::var("HOME") {
    for name in &binary_names {
      candidates.push(
        PathBuf::from(&home_dir)
          .join(".local")
          .join("bin")
          .join(name),
      );
    }
  }

  if let Ok(user_profile) = env::var("USERPROFILE") {
    for name in &binary_names {
      candidates.push(
        PathBuf::from(&user_profile)
          .join("AppData")
          .join("Local")
          .join("Cavell")
          .join("bin")
          .join(name),
      );
    }
  }

  candidates
}

fn default_model_candidates() -> Vec<PathBuf> {
  let file_names = ["LFM2.5-350M.gguf", "lfm2.5-350m.gguf"];
  let mut candidates = vec![];

  for current_dir in discovery_roots() {
    for name in &file_names {
      candidates.push(current_dir.join("models").join(name));
      candidates.push(current_dir.join("model-packs").join(name));
    }
  }

  if let Ok(home_dir) = env::var("HOME") {
    for name in &file_names {
      candidates.push(
        PathBuf::from(&home_dir)
          .join(".cavell")
          .join("models")
          .join(name),
      );
    }
  }

  if let Ok(user_profile) = env::var("USERPROFILE") {
    for name in &file_names {
      candidates.push(
        PathBuf::from(&user_profile)
          .join(".cavell")
          .join("models")
          .join(name),
      );
    }
  }

  candidates
}

fn read_manifest(path: &Path) -> Result<ModelPackManifest> {
  let content =
    fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
  serde_json::from_str(&content)
    .with_context(|| format!("failed to parse model pack manifest {}", path.display()))
}

fn discovery_roots() -> Vec<PathBuf> {
  let mut roots = vec![];

  if let Ok(current_executable) = env::current_exe() {
    if let Some(parent) = current_executable.parent() {
      roots.push(parent.to_path_buf());
    }
  }

  if let Ok(current_directory) = env::current_dir() {
    roots.push(current_directory);
  }

  let mut unique_roots = vec![];
  for root in roots {
    if !unique_roots.contains(&root) {
      unique_roots.push(root);
    }
  }

  unique_roots
}

fn model_metrics(manifest: Option<&ModelPackManifest>) -> HashMap<String, String> {
  let mut metrics = HashMap::new();
  if let Some(manifest) = manifest {
    metrics.insert("backend".to_string(), manifest.backend.clone());
    metrics.insert("contextSize".to_string(), manifest.context_size.to_string());
    metrics.insert(
      "maxOutputTokens".to_string(),
      manifest.max_output_tokens.to_string(),
    );
    metrics.insert("fileName".to_string(), manifest.file_name.clone());
  } else {
    metrics.insert("backend".to_string(), "llama.cpp".to_string());
    metrics.insert("contextSize".to_string(), "4096".to_string());
    metrics.insert("maxOutputTokens".to_string(), "160".to_string());
  }

  metrics
}

fn missing_runtime_detail(
  binary_path: Option<&Path>,
  model_path: Option<&Path>,
  manifest_path: Option<&Path>,
) -> String {
  match (binary_path, model_path, manifest_path) {
    (Some(binary_path), Some(model_path), Some(manifest_path)) => format!(
      "Falling back to the built-in heuristic summarizer because {} or {} is missing. Manifest: {}.",
      binary_path.display(),
      model_path.display(),
      manifest_path.display()
    ),
    (Some(binary_path), Some(model_path), None) => format!(
      "Falling back to the built-in heuristic summarizer because {} or {} is missing.",
      binary_path.display(),
      model_path.display()
    ),
    (Some(binary_path), None, Some(manifest_path)) => format!(
      "Falling back to the built-in heuristic summarizer because the model file is not configured or missing. Binary candidate: {}. Manifest: {}.",
      binary_path.display(),
      manifest_path.display()
    ),
    (Some(binary_path), None, None) => format!(
      "Falling back to the built-in heuristic summarizer because the model file is not configured or missing. Binary candidate: {}.",
      binary_path.display()
    ),
    (None, Some(model_path), Some(manifest_path)) => format!(
      "Falling back to the built-in heuristic summarizer because the llama.cpp CLI binary is not configured or missing. Model candidate: {}. Manifest: {}.",
      model_path.display(),
      manifest_path.display()
    ),
    (None, Some(model_path), None) => format!(
      "Falling back to the built-in heuristic summarizer because the llama.cpp CLI binary is not configured or missing. Model candidate: {}.",
      model_path.display()
    ),
    (None, None, Some(manifest_path)) => format!(
      "Falling back to the built-in heuristic summarizer because no llama.cpp CLI binary or resolved LFM2.5-350M model file is configured yet. Manifest: {}.",
      manifest_path.display()
    ),
    (None, None, None) => "Falling back to the built-in heuristic summarizer because no llama.cpp CLI binary or LFM2.5-350M model pack is configured yet.".to_string(),
  }
}

fn display_path(path: impl AsRef<Path>) -> String {
  path.as_ref().display().to_string()
}

fn generate_with_llama_cpp(
  binary_path: &Path,
  model_path: &Path,
  request: &GenerateRequest,
  manifest: Option<&ModelPackManifest>,
) -> Result<String> {
  let context_size = manifest.map(|item| item.context_size).unwrap_or(4096);
  let max_tokens = manifest
    .map(|item| item.max_output_tokens.min(request.max_tokens))
    .unwrap_or(request.max_tokens);
  let output = Command::new(binary_path)
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
    .arg(&request.prompt)
    .output()
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

fn fallback_from_prompt(prompt: &str, role: &ModelRole) -> String {
  let role_label = match role {
    ModelRole::Default => "default",
    ModelRole::Planner => "planner",
    ModelRole::Coder => "coder",
    ModelRole::Summarizer => "summarizer",
  };

  let preview = prompt
    .lines()
    .find(|line| !line.trim().is_empty())
    .unwrap_or("No prompt content was provided.");

  format!("Cavell used the local {role_label} fallback path. Prompt preview: {preview}")
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn runtime_uses_heuristic_backend_when_paths_are_missing() {
    let runtime = LocalModelRuntime::from_paths(None, None);
    let health = runtime.health();

    assert_eq!(health.display_name, "LFM2.5-350M");
    assert_eq!(health.backend, "heuristic");
    assert_eq!(health.status, "fallback");
    assert!(health.metrics.contains_key("contextSize"));
  }

  #[test]
  fn heuristic_generation_returns_fallback_text() {
    let runtime = LocalModelRuntime::from_paths(None, None);
    let response = runtime.generate(GenerateRequest {
      role: ModelRole::Summarizer,
      prompt: "Summarize this test prompt.".to_string(),
      fallback: "Fallback summary".to_string(),
      max_tokens: 96,
    });

    assert_eq!(response.text, "Fallback summary");
    assert_eq!(response.backend, "heuristic");
    assert_eq!(response.status, "fallback");
    assert_eq!(response.model_id, "lfm2.5-350m");
  }
}
