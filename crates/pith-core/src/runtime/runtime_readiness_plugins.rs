use pith_protocol::RuntimeReadinessCheck;

pub(super) fn plugin_check(
  enabled_plugin_count: usize,
  plugin_count: usize,
  enabled_command_count: usize,
  command_count: usize,
) -> RuntimeReadinessCheck {
  let status = if enabled_command_count > 0 {
    "ready"
  } else if command_count > 0 {
    "setup_required"
  } else {
    "optional"
  };

  RuntimeReadinessCheck {
    id: "plugins".to_string(),
    title: "Plugins".to_string(),
    status: status.to_string(),
    detail: format!(
      "{enabled_plugin_count} enabled of {plugin_count} discovered plugin(s); \
       {enabled_command_count} enabled of {command_count} command capability(s)."
    ),
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn plugin_check_requires_enabled_command_capability_for_ready_status() {
    let check = plugin_check(1, 1, 0, 1);

    assert_eq!(check.status, "setup_required");
    assert!(check.detail.contains("0 enabled of 1 command capability"));
  }

  #[test]
  fn plugin_check_reports_ready_with_enabled_command_capability() {
    let check = plugin_check(2, 3, 1, 2);

    assert_eq!(check.status, "ready");
    assert!(check.detail.contains("1 enabled of 2 command capability"));
  }

  #[test]
  fn plugin_check_stays_optional_without_command_capabilities() {
    let check = plugin_check(1, 1, 0, 0);

    assert_eq!(check.status, "optional");
  }
}
