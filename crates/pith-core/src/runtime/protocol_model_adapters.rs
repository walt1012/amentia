use pith_model_runtime::{ModelBootstrap, ModelHealth};
use pith_protocol::{ModelBootstrapResult, ModelHealthResult};

pub(crate) fn to_protocol_model_health(health: ModelHealth) -> ModelHealthResult {
  ModelHealthResult {
    pack_id: health.pack_id,
    display_name: health.display_name,
    backend: health.backend,
    status: health.status,
    detail: health.detail,
    source: health.source,
    binary_path: health.binary_path,
    model_path: health.model_path,
    manifest_path: health.manifest_path,
    metrics: health.metrics,
  }
}

pub(crate) fn to_protocol_model_bootstrap(result: ModelBootstrap) -> ModelBootstrapResult {
  ModelBootstrapResult {
    manifest_path: result.manifest_path.display().to_string(),
    readme_path: result.readme_path.map(|path| path.display().to_string()),
    copied_files: result
      .copied_files
      .into_iter()
      .map(|path| path.display().to_string())
      .collect(),
  }
}
