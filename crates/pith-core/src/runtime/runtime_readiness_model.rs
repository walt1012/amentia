use pith_protocol::RuntimeReadinessCheck;

pub(super) fn local_model_check(
  model_ready: bool,
  display_name: &str,
  backend: &str,
  health_detail: &str,
) -> RuntimeReadinessCheck {
  let status = if model_ready {
    "ready"
  } else {
    "setup_required"
  };

  RuntimeReadinessCheck {
    id: "localModel".to_string(),
    title: "Local Model".to_string(),
    status: status.to_string(),
    detail: if model_ready {
      format!("{display_name} is ready through {backend}.")
    } else if health_detail.trim().is_empty() {
      "Download and select one local model before agent work starts.".to_string()
    } else {
      health_detail.to_string()
    },
  }
}

pub(super) fn context_check(context_window: &str, output_cap: &str) -> RuntimeReadinessCheck {
  RuntimeReadinessCheck {
    id: "context".to_string(),
    title: "Context".to_string(),
    status: "ready".to_string(),
    detail: format!(
      "Context packing uses a {context_window} token runtime window with {output_cap} output cap."
    ),
  }
}
