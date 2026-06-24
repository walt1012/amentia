use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::ModelPackManifest;

#[derive(Debug, Clone)]
pub(crate) struct ManifestResolution {
  pub(crate) manifest: ModelPackManifest,
  pub(crate) manifest_path: PathBuf,
  pub(crate) source: String,
}

pub(crate) fn resolve_binary_path() -> Option<PathBuf> {
  if let Ok(path) = env::var("AMENTIA_LLAMACPP_PATH") {
    return Some(PathBuf::from(path));
  }

  if require_packaged_llama_backend() {
    return None;
  }

  default_binary_candidates()
    .into_iter()
    .find(|path| path.is_file())
}

pub(crate) fn resolve_model_path(
  manifest_directory: Option<PathBuf>,
  manifest: Option<&ModelPackManifest>,
) -> Option<PathBuf> {
  if let Ok(path) = env::var("AMENTIA_MODEL_PATH") {
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

pub(crate) fn resolve_manifest() -> Option<ManifestResolution> {
  let env_manifest = env::var("AMENTIA_MODEL_PACK_MANIFEST")
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
      "default-manifest".to_string(),
    ));
    candidates.push((
      current_dir
        .join("model-packs")
        .join("lfm2.5-350m")
        .join("model-pack.json"),
      "default-manifest".to_string(),
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

pub(crate) fn resolve_bootstrap_manifest(
  target_manifest_path: &Path,
) -> Option<ManifestResolution> {
  let resolution = resolve_manifest()?;
  if normalize_path(&resolution.manifest_path) != normalize_path(target_manifest_path) {
    return Some(resolution);
  }

  for current_dir in discovery_roots() {
    let candidates = [
      current_dir
        .join("models")
        .join("builtin")
        .join("lfm2.5-350m")
        .join("model-pack.json"),
      current_dir
        .join("model-packs")
        .join("lfm2.5-350m")
        .join("model-pack.json"),
    ];

    for path in candidates {
      if normalize_path(&path) == normalize_path(target_manifest_path) || !path.is_file() {
        continue;
      }

      if let Ok(manifest) = read_manifest(&path) {
        return Some(ManifestResolution {
          manifest,
          manifest_path: path,
          source: "default-manifest".to_string(),
        });
      }
    }
  }

  Some(resolution)
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
          .join("Amentia")
          .join("bin")
          .join(name),
      );
    }
  }

  candidates
}

fn default_model_candidates() -> Vec<PathBuf> {
  let file_names = [
    "LFM2.5-350M-Q4_K_M.gguf",
    "LFM2.5-350M.gguf",
    "lfm2.5-350m.gguf",
  ];
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
          .join(".amentia")
          .join("models")
          .join(name),
      );
    }
  }

  if let Ok(user_profile) = env::var("USERPROFILE") {
    for name in &file_names {
      candidates.push(
        PathBuf::from(&user_profile)
          .join(".amentia")
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

fn require_packaged_llama_backend() -> bool {
  env::var("AMENTIA_REQUIRE_PACKAGED_LLAMACPP")
    .map(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"))
    .unwrap_or(false)
}

pub(crate) fn normalize_path(path: &Path) -> PathBuf {
  path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

pub(crate) fn discovery_roots() -> Vec<PathBuf> {
  let mut roots = vec![];

  if let Ok(model_pack_root) = env::var("AMENTIA_MODEL_PACK_ROOT") {
    roots.push(PathBuf::from(model_pack_root));
  }

  if let Ok(data_dir) = env::var("AMENTIA_DATA_DIR") {
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

pub(crate) fn suggested_manifest_install_path() -> PathBuf {
  suggested_model_root()
    .join("builtin")
    .join("lfm2.5-350m")
    .join("model-pack.json")
}

pub(crate) fn suggested_model_install_path(file_name: &str) -> PathBuf {
  suggested_model_root()
    .join("builtin")
    .join("lfm2.5-350m")
    .join(file_name)
}

pub(crate) fn suggested_binary_install_path() -> PathBuf {
  let binary_name = if cfg!(windows) {
    "llama-cli.exe"
  } else {
    "llama-cli"
  };

  if let Ok(data_dir) = env::var("AMENTIA_DATA_DIR") {
    return PathBuf::from(data_dir).join("bin").join(binary_name);
  }

  if cfg!(windows) {
    if let Ok(user_profile) = env::var("USERPROFILE") {
      return PathBuf::from(user_profile)
        .join("AppData")
        .join("Local")
        .join("Amentia")
        .join("bin")
        .join(binary_name);
    }
  }

  if let Ok(home_dir) = env::var("HOME") {
    return PathBuf::from(home_dir)
      .join(".amentia")
      .join("bin")
      .join(binary_name);
  }

  PathBuf::from("bin").join(binary_name)
}

fn suggested_model_root() -> PathBuf {
  if let Ok(data_dir) = env::var("AMENTIA_DATA_DIR") {
    return PathBuf::from(data_dir).join("models");
  }

  if cfg!(windows) {
    if let Ok(user_profile) = env::var("USERPROFILE") {
      return PathBuf::from(user_profile)
        .join("AppData")
        .join("Local")
        .join("Amentia")
        .join("models");
    }
  }

  if let Ok(home_dir) = env::var("HOME") {
    return PathBuf::from(home_dir).join(".amentia").join("models");
  }

  PathBuf::from("models")
}
