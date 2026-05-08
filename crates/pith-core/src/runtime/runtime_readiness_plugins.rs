use pith_protocol::RuntimeReadinessCheck;

pub(super) fn plugin_check(
  enabled_plugin_count: usize,
  plugin_count: usize,
) -> RuntimeReadinessCheck {
  let status = if enabled_plugin_count > 0 {
    "ready"
  } else {
    "optional"
  };

  RuntimeReadinessCheck {
    id: "plugins".to_string(),
    title: "Plugins".to_string(),
    status: status.to_string(),
    detail: format!("{enabled_plugin_count} enabled of {plugin_count} discovered plugin(s)."),
  }
}
