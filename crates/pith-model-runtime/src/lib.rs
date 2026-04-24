use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

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
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub license: Option<String>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub homepage: Option<String>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub sha256: Option<String>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub size_bytes: Option<u64>,
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
pub struct ModelBootstrap {
  pub manifest_path: PathBuf,
  pub readme_path: Option<PathBuf>,
  pub copied_files: Vec<PathBuf>,
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
          backend: "heuristic".to_string(),
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
    match &self.backend {
      ModelBackend::Heuristic { detail, .. } => {
        self.generate_failure("heuristic", "unavailable", &request.role, detail)
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
        Err(error) => self.generate_failure(
          "llama.cpp",
          "error",
          &request.role,
          &format!("Local llama.cpp inference failed: {error}"),
        ),
      },
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
    let resolution =
      resolve_manifest().context("failed to locate a bundled model-pack.json for bootstrap")?;
    let target_manifest_path = suggested_manifest_install_path();
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
  if let Ok(path) = env::var("PITH_LLAMACPP_PATH") {
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
  if let Ok(path) = env::var("PITH_LFM_MODEL_PATH") {
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
  let env_manifest = env::var("PITH_MODEL_PACK_MANIFEST").ok().map(PathBuf::from);
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
          .join("Pith")
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
          .join(".pith")
          .join("models")
          .join(name),
      );
    }
  }

  if let Ok(user_profile) = env::var("USERPROFILE") {
    for name in &file_names {
      candidates.push(
        PathBuf::from(&user_profile)
          .join(".pith")
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

fn normalize_path(path: &Path) -> PathBuf {
  path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

fn discovery_roots() -> Vec<PathBuf> {
  let mut roots = vec![];

  if let Ok(model_pack_root) = env::var("PITH_MODEL_PACK_ROOT") {
    roots.push(PathBuf::from(model_pack_root));
  }

  if let Ok(data_dir) = env::var("PITH_DATA_DIR") {
    roots.push(PathBuf::from(&data_dir));
    roots.push(PathBuf::from(&data_dir).join("models"));
  }

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

fn model_metrics(
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
    .unwrap_or("LFM2.5-350M.gguf");
  let suggested_manifest_path = suggested_manifest_install_path();
  let suggested_model_path = suggested_model_install_path(suggested_file_name);
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
    .unwrap_or("LFM2.5-350M.gguf");
  let suggested_manifest = suggested_manifest_install_path();
  let suggested_model = suggested_model_install_path(file_name);
  let suggested_binary = suggested_binary_install_path();

  match readiness {
    "ready" => format!(
      "Local inference is ready. Keep the manifest near {} and use {} if you need to override discovery.",
      file_name,
      "PITH_LFM_MODEL_PATH"
    ),
    "model_missing" => format!(
      "Place {} at {} or set PITH_LFM_MODEL_PATH. Current binary candidate: {}.",
      file_name,
      suggested_model.display(),
      binary_path
        .map(display_path)
        .unwrap_or_else(|| suggested_binary.display().to_string())
    ),
    "binary_missing" => format!(
      "Install llama.cpp CLI at {} or set PITH_LLAMACPP_PATH. Current model candidate: {}.",
      suggested_binary.display(),
      model_path
        .map(display_path)
        .unwrap_or_else(|| suggested_model.display().to_string())
    ),
    "manifest_only" => format!(
      "Keep the manifest at {} and place {} beside it. Then install llama.cpp CLI at {} or set PITH_MODEL_PACK_MANIFEST, PITH_LFM_MODEL_PATH, and PITH_LLAMACPP_PATH.",
      manifest_path
        .map(display_path)
        .unwrap_or_else(|| suggested_manifest.display().to_string()),
      file_name,
      suggested_binary.display()
    ),
    "misconfigured" => format!(
      "Resolved candidates exist but local inference is not ready. Verify the manifest at {}, model at {}, and llama.cpp CLI at {}.",
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
      "Add model-pack.json at {}, place {} beside it, and install llama.cpp CLI at {}. You can also set PITH_MODEL_PACK_MANIFEST, PITH_LFM_MODEL_PATH, and PITH_LLAMACPP_PATH directly.",
      suggested_manifest.display(),
      file_name,
      suggested_binary.display()
    ),
  }
}

fn suggested_manifest_install_path() -> PathBuf {
  suggested_model_root()
    .join("builtin")
    .join("lfm2.5-350m")
    .join("model-pack.json")
}

fn suggested_model_install_path(file_name: &str) -> PathBuf {
  suggested_model_root()
    .join("builtin")
    .join("lfm2.5-350m")
    .join(file_name)
}

fn suggested_binary_install_path() -> PathBuf {
  let binary_name = if cfg!(windows) {
    "llama-cli.exe"
  } else {
    "llama-cli"
  };

  if let Ok(data_dir) = env::var("PITH_DATA_DIR") {
    return PathBuf::from(data_dir).join("bin").join(binary_name);
  }

  if cfg!(windows) {
    if let Ok(user_profile) = env::var("USERPROFILE") {
      return PathBuf::from(user_profile)
        .join("AppData")
        .join("Local")
        .join("Pith")
        .join("bin")
        .join(binary_name);
    }
  }

  if let Ok(home_dir) = env::var("HOME") {
    return PathBuf::from(home_dir)
      .join(".pith")
      .join("bin")
      .join(binary_name);
  }

  PathBuf::from("bin").join(binary_name)
}

fn suggested_model_root() -> PathBuf {
  if let Ok(data_dir) = env::var("PITH_DATA_DIR") {
    return PathBuf::from(data_dir).join("models");
  }

  if cfg!(windows) {
    if let Ok(user_profile) = env::var("USERPROFILE") {
      return PathBuf::from(user_profile)
        .join("AppData")
        .join("Local")
        .join("Pith")
        .join("models");
    }
  }

  if let Ok(home_dir) = env::var("HOME") {
    return PathBuf::from(home_dir).join(".pith").join("models");
  }

  PathBuf::from("models")
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

fn generation_failure_text(role: &ModelRole, detail: &str) -> String {
  let role_label = match role {
    ModelRole::Default => "default",
    ModelRole::Planner => "planner",
    ModelRole::Coder => "coder",
    ModelRole::Summarizer => "summarizer",
  };

  format!(
    "Pith could not produce a local {role_label} response because {detail}"
  )
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::io::ErrorKind;
  use std::path::PathBuf;
  use std::time::{SystemTime, UNIX_EPOCH};

  #[test]
  fn runtime_uses_heuristic_backend_when_paths_are_missing() {
    let runtime = LocalModelRuntime::from_paths(None, None);
    let health = runtime.health();

    assert_eq!(health.display_name, "LFM2.5-350M");
    assert_eq!(health.backend, "heuristic");
    assert_eq!(health.status, "unavailable");
    assert!(health.metrics.contains_key("contextSize"));
    assert_eq!(health.metrics["readiness"], "unconfigured");
    assert_eq!(health.metrics["packReady"], "false");
    assert!(health.metrics["installHint"].contains("model-pack.json"));
    assert!(health.metrics.contains_key("suggestedManifestPath"));
    assert!(health.metrics.contains_key("suggestedModelPath"));
    assert!(health.metrics.contains_key("suggestedBinaryPath"));
  }

  #[test]
  fn heuristic_generation_returns_unavailable_error() {
    let runtime = LocalModelRuntime::from_paths(None, None);
    let response = runtime.generate(GenerateRequest {
      role: ModelRole::Summarizer,
      prompt: "Summarize this test prompt.".to_string(),
      fallback: "Fallback summary".to_string(),
      max_tokens: 96,
    });

    assert!(response
      .text
      .contains("could not produce a local summarizer response"));
    assert_eq!(response.backend, "heuristic");
    assert_eq!(response.status, "unavailable");
    assert_eq!(response.model_id, "lfm2.5-350m");
  }

  #[test]
  fn discovery_roots_include_configured_model_directories() {
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
  "display_name": "LFM2.5-350M",
  "file_name": "LFM2.5-350M.gguf",
  "context_size": 4096,
  "max_output_tokens": 160,
  "backend": "llama.cpp"
}"#,
    )
    .expect("manifest");
    fs::write(
      source_pack_root.join("README.md"),
      "Built-in model pack metadata",
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

  fn restore_env_var(key: &str, value: Option<String>) {
    match value {
      Some(value) => env::set_var(key, value),
      None => env::remove_var(key),
    }
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
