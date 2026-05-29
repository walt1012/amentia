use std::collections::HashMap;

use pith_protocol::TimelineItem;
use pith_tools::{web_search_timeout_seconds, WebSearchStatus};

use super::turn_tool_limits::WEB_SEARCH_RESULT_LIMIT;
use super::turn_tool_provenance::web_tool_attributes;
use crate::context::local_response_web_search::WEB_SEARCH_SOURCE_MODE;
use crate::intent_inference::WebSearchIntent;

pub(super) fn web_search_start_item(
  intent: &WebSearchIntent,
  status: &WebSearchStatus,
) -> TimelineItem {
  TimelineItem {
    kind: "toolStart".to_string(),
    title: "web_search".to_string(),
    content: intent.query.clone(),
    attributes: Some(web_search_attributes(intent, status)),
  }
}

pub(super) fn web_search_result_item(
  intent: &WebSearchIntent,
  status: &WebSearchStatus,
  content: String,
  result_count: usize,
) -> TimelineItem {
  TimelineItem {
    kind: "toolResult".to_string(),
    title: "web_search result".to_string(),
    content,
    attributes: Some(with_web_search_result_count(
      web_search_attributes(intent, status),
      result_count,
    )),
  }
}

pub(super) fn web_search_unavailable_items(
  intent: &WebSearchIntent,
  status: &WebSearchStatus,
) -> Vec<TimelineItem> {
  vec![
    TimelineItem {
      kind: "warning".to_string(),
      title: "web_search unavailable".to_string(),
      content: status.detail.clone(),
      attributes: Some(web_search_attributes(intent, status)),
    },
    TimelineItem {
      kind: "assistantMessage".to_string(),
      title: "Assistant".to_string(),
      content: "Pith could not search the web because the built-in search client is unavailable."
        .to_string(),
      attributes: None,
    },
  ]
}

pub(super) fn web_search_failed_items(
  intent: &WebSearchIntent,
  status: &WebSearchStatus,
  error: String,
) -> Vec<TimelineItem> {
  vec![
    TimelineItem {
      kind: "warning".to_string(),
      title: "web_search failed".to_string(),
      content: error,
      attributes: Some(web_search_attributes(intent, status)),
    },
    TimelineItem {
      kind: "assistantMessage".to_string(),
      title: "Assistant".to_string(),
      content: "Pith could not search the web yet. Check network access and try again.".to_string(),
      attributes: None,
    },
  ]
}

fn web_search_attributes(
  intent: &WebSearchIntent,
  status: &WebSearchStatus,
) -> HashMap<String, String> {
  web_tool_attributes(
    "web_search",
    [
      ("query".to_string(), intent.query.clone()),
      (
        "maxResults".to_string(),
        WEB_SEARCH_RESULT_LIMIT.to_string(),
      ),
      ("provider".to_string(), status.provider.clone()),
      ("client".to_string(), status.client.clone()),
      ("networkAccess".to_string(), "true".to_string()),
      (
        "routingReason".to_string(),
        intent.routing_reason.to_string(),
      ),
      (
        "timeoutSeconds".to_string(),
        web_search_timeout_seconds().to_string(),
      ),
      (
        "webSearchAvailable".to_string(),
        status.available.to_string(),
      ),
      (
        "webSearchSourceMode".to_string(),
        WEB_SEARCH_SOURCE_MODE.to_string(),
      ),
      ("pageFetchPerformed".to_string(), "false".to_string()),
      ("sourceSnapshotAvailable".to_string(), "false".to_string()),
      ("sourceSnapshotKind".to_string(), "none".to_string()),
    ],
  )
}

fn with_web_search_result_count(
  mut attributes: HashMap<String, String>,
  result_count: usize,
) -> HashMap<String, String> {
  attributes.insert("resultCount".to_string(), result_count.to_string());
  attributes
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn web_search_attributes_use_runtime_status() {
    let intent = WebSearchIntent {
      query: "latest pith release".to_string(),
      routing_reason: "freshPublicInformation",
    };
    let status = WebSearchStatus {
      provider: "Example Search".to_string(),
      client: "example-client".to_string(),
      available: true,
      detail: "ready".to_string(),
    };

    let attributes = web_search_attributes(&intent, &status);

    assert_eq!(
      attributes.get("tool").map(String::as_str),
      Some("web_search")
    );
    assert_eq!(
      attributes.get("toolSchema").map(String::as_str),
      Some("pith.localTool.v1")
    );
    assert_eq!(attributes.get("toolKind").map(String::as_str), Some("web"));
    assert_eq!(
      attributes.get("actionBoundary").map(String::as_str),
      Some("network")
    );
    assert_eq!(
      attributes.get("actionApprovalPolicy").map(String::as_str),
      Some("requiresPluginPermission")
    );
    assert_eq!(
      attributes.get("provider").map(String::as_str),
      Some("Example Search")
    );
    assert_eq!(
      attributes.get("query").map(String::as_str),
      Some("latest pith release")
    );
    assert_eq!(attributes.get("maxResults").map(String::as_str), Some("5"));
    assert_eq!(
      attributes.get("client").map(String::as_str),
      Some("example-client")
    );
    assert_eq!(
      attributes.get("routingReason").map(String::as_str),
      Some("freshPublicInformation")
    );
    assert_eq!(
      attributes.get("webSearchAvailable").map(String::as_str),
      Some("true")
    );
    assert_eq!(
      attributes.get("webSearchSourceMode").map(String::as_str),
      Some("searchResultAttribution")
    );
    assert_eq!(
      attributes.get("pageFetchPerformed").map(String::as_str),
      Some("false")
    );
    assert_eq!(
      attributes
        .get("sourceSnapshotAvailable")
        .map(String::as_str),
      Some("false")
    );
    assert_eq!(
      attributes.get("sourceSnapshotKind").map(String::as_str),
      Some("none")
    );
    let timeout_seconds = web_search_timeout_seconds().to_string();
    assert_eq!(attributes.get("timeoutSeconds"), Some(&timeout_seconds));
  }

  #[test]
  fn web_search_result_count_extends_attributes() {
    let attributes = with_web_search_result_count(HashMap::new(), 3);

    assert_eq!(attributes.get("resultCount").map(String::as_str), Some("3"));
  }
}
