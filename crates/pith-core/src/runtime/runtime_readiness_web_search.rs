use pith_protocol::RuntimeReadinessCheck;
use pith_tools::WebSearchStatus;

pub(super) fn web_search_check(
  status: &WebSearchStatus,
  permission_sources: &[String],
) -> RuntimeReadinessCheck {
  let check_status = if permission_sources.is_empty() {
    "setup_required"
  } else if status.available {
    "ready"
  } else {
    "limited"
  };
  let detail = if permission_sources.is_empty() {
    "Enable Web Search in Connectors to allow current-information lookup.".to_string()
  } else {
    format!(
      "{} Permission granted by {}.",
      status.detail,
      permission_sources.join(", ")
    )
  };

  RuntimeReadinessCheck {
    id: "webSearch".to_string(),
    title: "Web Search".to_string(),
    status: check_status.to_string(),
    detail,
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn status(available: bool) -> WebSearchStatus {
    WebSearchStatus {
      available,
      provider: "DuckDuckGo Lite".to_string(),
      client: "curl".to_string(),
      detail: "Web search tool is available.".to_string(),
    }
  }

  #[test]
  fn web_search_check_requires_web_search_tool_permission() {
    let check = web_search_check(&status(true), &[]);

    assert_eq!(check.status, "setup_required");
    assert!(check.detail.contains("Enable Web Search in Connectors"));
  }

  #[test]
  fn web_search_check_reports_ready_with_permission_and_available_tool() {
    let check = web_search_check(&status(true), &["Web Search".to_string()]);

    assert_eq!(check.status, "ready");
    assert!(check.detail.contains("Permission granted by Web Search"));
  }

  #[test]
  fn web_search_check_reports_limited_when_tool_is_missing() {
    let check = web_search_check(&status(false), &["Web Search".to_string()]);

    assert_eq!(check.status, "limited");
  }
}
