use amentia_protocol::RuntimeReadinessCheck;
use amentia_sandbox::NativeSandboxStatus;

pub(super) fn native_sandbox_check(status: &NativeSandboxStatus) -> RuntimeReadinessCheck {
  let check_status = if status.active {
    "ready"
  } else if status.available {
    "setup_required"
  } else {
    "limited"
  };

  RuntimeReadinessCheck {
    id: "nativeSandbox".to_string(),
    title: "Native Sandbox".to_string(),
    status: check_status.to_string(),
    detail: status.detail.clone(),
  }
}
