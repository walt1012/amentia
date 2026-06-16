use std::time::Duration;

use pith_model_runtime::{
  GenerateRequest, GenerateResponse, LocalModelRuntime, ModelBootstrap, ModelHealth, ModelRole,
};

const MODEL_PROBE_PROMPT: &str =
  "Reply with exactly this short phrase if local generation is working: Pith model ready.";
const MODEL_PROBE_TIMEOUT: Duration = Duration::from_secs(45);

#[derive(Debug, Clone)]
pub(crate) struct RuntimeModelState {
  runtime: LocalModelRuntime,
  enforce_readiness: bool,
}

impl RuntimeModelState {
  pub(crate) fn new_default(enforce_readiness: bool) -> Self {
    Self {
      runtime: LocalModelRuntime::new_default(),
      enforce_readiness,
    }
  }

  pub(crate) fn runtime(&self) -> &LocalModelRuntime {
    &self.runtime
  }

  pub(crate) fn snapshot(&self) -> LocalModelRuntime {
    self.runtime.clone()
  }

  pub(crate) fn health(&self) -> ModelHealth {
    self.runtime.health()
  }

  pub(crate) fn probe(&self) -> GenerateResponse {
    self.runtime.generate(GenerateRequest {
      role: ModelRole::Summarizer,
      prompt: MODEL_PROBE_PROMPT.to_string(),
      max_tokens: 16,
      timeout: Some(MODEL_PROBE_TIMEOUT),
      cancellation: None,
    })
  }

  pub(crate) fn bootstrap_pack_metadata(&self) -> anyhow::Result<ModelBootstrap> {
    self.runtime.bootstrap_pack_metadata()
  }

  pub(crate) fn reset_default(&mut self) {
    self.runtime = LocalModelRuntime::new_default();
  }

  #[cfg(test)]
  pub(crate) fn set_enforce_readiness(&mut self, enforce_readiness: bool) {
    self.enforce_readiness = enforce_readiness;
  }

  pub(crate) fn ensure_ready_for_turn(&self) -> std::result::Result<(), String> {
    if !self.enforce_readiness {
      return Ok(());
    }

    let health = self.health();
    if health.status == "ready" {
      return Ok(());
    }

    Err(format!(
      "Local model is not ready for turn/start. Download and activate a local model first. {}",
      health.detail
    ))
  }
}
