use pith_model_runtime::{LocalModelRuntime, ModelBootstrap, ModelHealth};

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
