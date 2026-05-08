use pith_protocol::RuntimeReadinessCheck;
use pith_tools::WebSearchStatus;

pub(super) fn web_search_check(status: &WebSearchStatus) -> RuntimeReadinessCheck {
  let check_status = if status.available { "ready" } else { "limited" };

  RuntimeReadinessCheck {
    id: "webSearch".to_string(),
    title: "Web Search".to_string(),
    status: check_status.to_string(),
    detail: status.detail.clone(),
  }
}
