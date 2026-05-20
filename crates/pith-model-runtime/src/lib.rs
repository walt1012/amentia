use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

mod discovery;
mod health;
mod inference;
#[cfg(test)]
mod tests;
mod types;
mod validation;

pub use inference::llama_cpp_timeout_seconds;
pub use types::*;

use discovery::{
  normalize_path, resolve_binary_path, resolve_bootstrap_manifest, resolve_manifest,
  resolve_model_path, suggested_manifest_install_path, ManifestResolution,
};
use health::{display_path, missing_runtime_detail, model_metrics};
use inference::{generate_with_llama_cpp, generation_failure_text, request_is_cancelled};
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
