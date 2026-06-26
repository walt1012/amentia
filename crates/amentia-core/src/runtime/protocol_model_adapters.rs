use amentia_model_runtime::{GenerateResponse, ModelBootstrap, ModelHealth};
use amentia_protocol::{ModelBootstrapResult, ModelHealthResult, ModelProbeResult};

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

pub(crate) fn to_protocol_model_probe(response: GenerateResponse) -> ModelProbeResult {
  let is_ready = response.status == "ready";
  let detail = if is_ready {
    "The active local model answered a short probe.".to_string()
  } else {
    response.detail.clone().unwrap_or_else(|| response.text.clone())
  };
  let sample = if is_ready {
    Some(compact_probe_sample(&response.text))
  } else {
    None
  };

  ModelProbeResult {
    status: response.status,
    detail,
    backend: response.backend,
    model_id: response.model_id,
    sample,
  }
}

fn compact_probe_sample(text: &str) -> String {
  let trimmed = text.trim();
  if trimmed.chars().count() <= 120 {
    return trimmed.to_string();
  }

  trimmed.chars().take(117).collect::<String>() + "..."
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn model_probe_ready_result_keeps_compact_sample() {
    let result = to_protocol_model_probe(GenerateResponse {
      text: "Amentia model ready.".to_string(),
      backend: "llama.cpp".to_string(),
      status: "ready".to_string(),
      model_id: "test-model".to_string(),
      detail: None,
    });

    assert_eq!(result.status, "ready");
    assert_eq!(
      result.detail,
      "The active local model answered a short probe."
    );
    assert_eq!(result.sample, Some("Amentia model ready.".to_string()));
  }

  #[test]
  fn model_probe_failure_result_keeps_repair_detail_without_sample() {
    let result = to_protocol_model_probe(GenerateResponse {
      text: "Amentia could not finish the local response.".to_string(),
      backend: "unconfigured".to_string(),
      status: "unavailable".to_string(),
      model_id: "test-model".to_string(),
      detail: Some("Packaged backend missing.".to_string()),
    });

    assert_eq!(result.status, "unavailable");
    assert_eq!(result.detail, "Packaged backend missing.");
    assert_eq!(result.sample, None);
  }
}
