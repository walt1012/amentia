use super::test_support::{
  bundled_plugin_entry, create_temp_workspace, enable_full_access_plugin, remove_temp_workspace,
  replace_plugin_catalog, request,
};
use super::*;
use amentia_protocol::methods;
use serde_json::json;
use std::ffi::OsString;
use std::fs;

#[test]
fn turn_start_web_search_uses_enabled_web_search_permission() {
  let mut context = RuntimeContext::new_in_memory();
  enable_web_search_plugin(&mut context);

  let _ = handle_request(
    &mut context,
    request(
      methods::THREAD_START,
      Some(json!({
        "title": "Web Thread"
      })),
    ),
  );
  let _ = handle_request(
    &mut context,
    request(
      methods::TURN_CANCEL_RUNNING,
      Some(json!({
        "threadId": "thread-1"
      })),
    ),
  );

  let turn_response = handle_request(
    &mut context,
    request(
      methods::TURN_START,
      Some(json!({
        "threadId": "thread-1",
        "message": "web search for Amentia local model"
      })),
    ),
  );

  assert!(turn_response.error.is_none());
  let result = turn_response.result.expect("turn result");
  let items = result["items"].as_array().expect("items");

  assert_eq!(items[0]["kind"], "userMessage");
  assert_eq!(items[1]["kind"], "plan");
  assert_eq!(
    items[1]["attributes"]["agentLoopSchema"],
    "amentia.agentLoop.v1"
  );
  assert_eq!(items[1]["attributes"]["agentToolKind"], "web");
  assert_eq!(items[1]["attributes"]["agentToolName"], "web_search");
  assert_eq!(items[2]["kind"], "warning");
  assert_eq!(items[2]["title"], "Turn Cancelled");
  assert!(items
    .iter()
    .all(|item| item["title"] != "Plugin Permission Required"));
}

#[test]
fn turn_start_web_search_respects_disabled_web_search_plugin() {
  let mut context = RuntimeContext::new_in_memory();
  replace_plugin_catalog(&mut context, vec![web_search_plugin(false)]);

  let _ = handle_request(
    &mut context,
    request(
      methods::THREAD_START,
      Some(json!({
        "title": "Disabled Web Thread"
      })),
    ),
  );

  let turn_response = handle_request(
    &mut context,
    request(
      methods::TURN_START,
      Some(json!({
        "threadId": "thread-1",
        "message": "web search for Amentia local model"
      })),
    ),
  );

  assert!(turn_response.error.is_none());
  let result = turn_response.result.expect("turn result");
  let items = result["items"].as_array().expect("items");
  let permission_item = items
    .iter()
    .find(|item| item["title"] == "Plugin Permission Required")
    .expect("permission item");

  assert_eq!(
    permission_item["attributes"]["requiredPermission"],
    "tool:web_search"
  );
  assert_eq!(permission_item["attributes"]["pluginId"], "web-search");
  assert_eq!(
    permission_item["attributes"]["permissionGate"],
    "requiresPluginPermission"
  );
  assert_eq!(
    permission_item["attributes"]["requiredPermissionLabel"],
    "Web Search"
  );
  assert!(permission_item["content"]
    .as_str()
    .expect("content")
    .contains("Web Search is not enabled"));
  assert_eq!(permission_item["attributes"]["grantedBy"], "none");
}

#[test]
fn turn_start_web_search_requires_web_search_tool_permission_not_any_network_plugin() {
  let mut context = RuntimeContext::new_in_memory();
  replace_plugin_catalog(
    &mut context,
    vec![bundled_plugin_entry(
      "notion-connector",
      "Notion Connector",
      true,
      false,
      &["connector:notion"],
      &["network.outbound"],
    )],
  );

  let _ = handle_request(
    &mut context,
    request(
      methods::THREAD_START,
      Some(json!({
        "title": "Connector Network Thread"
      })),
    ),
  );

  let turn_response = handle_request(
    &mut context,
    request(
      methods::TURN_START,
      Some(json!({
        "threadId": "thread-1",
        "message": "web search for Amentia local model"
      })),
    ),
  );

  assert!(turn_response.error.is_none());
  let result = turn_response.result.expect("turn result");
  let items = result["items"].as_array().expect("items");
  let permission_item = items
    .iter()
    .find(|item| item["title"] == "Plugin Permission Required")
    .expect("permission item");

  assert_eq!(
    permission_item["attributes"]["requiredPermission"],
    "tool:web_search"
  );
  assert_eq!(permission_item["attributes"]["pluginId"], "web-search");
  assert_eq!(
    permission_item["attributes"]["permissionGate"],
    "requiresPluginPermission"
  );
  assert_eq!(
    permission_item["attributes"]["requiredPermissionLabel"],
    "Web Search"
  );
  assert_eq!(permission_item["attributes"]["grantedBy"], "none");
}

#[test]
fn turn_start_routes_fresh_public_requests_to_enabled_web_search() {
  let mut context = RuntimeContext::new_in_memory();
  enable_web_search_plugin(&mut context);

  let _ = handle_request(
    &mut context,
    request(
      methods::THREAD_START,
      Some(json!({
        "title": "Fresh Info Thread"
      })),
    ),
  );
  let _ = handle_request(
    &mut context,
    request(
      methods::TURN_CANCEL_RUNNING,
      Some(json!({
        "threadId": "thread-1"
      })),
    ),
  );

  let turn_response = handle_request(
    &mut context,
    request(
      methods::TURN_START,
      Some(json!({
        "threadId": "thread-1",
        "message": "What is the latest Granite 4.0-H-350M release?"
      })),
    ),
  );

  assert!(turn_response.error.is_none());
  let result = turn_response.result.expect("turn result");
  let items = result["items"].as_array().expect("items");

  assert_eq!(items[0]["kind"], "userMessage");
  assert_eq!(items[1]["kind"], "plan");
  assert_eq!(items[2]["kind"], "warning");
  assert_eq!(items[2]["title"], "Turn Cancelled");
  assert!(items
    .iter()
    .all(|item| item["title"] != "Plugin Permission Required"));
}

#[test]
fn turn_start_prefers_workspace_file_over_fresh_web_search() {
  let mut context = RuntimeContext::new_in_memory();
  enable_full_access_plugin(&mut context);
  let workspace = create_temp_workspace("fresh-local-file");
  fs::write(
    workspace.join("Cargo.toml"),
    "[package]\nname = \"amentia-test\"\nversion = \"0.1.0\"\n",
  )
  .expect("write cargo manifest");

  let _ = handle_request(
    &mut context,
    request(
      methods::WORKSPACE_OPEN,
      Some(json!({
        "path": workspace.display().to_string()
      })),
    ),
  );
  let _ = handle_request(
    &mut context,
    request(
      methods::THREAD_START,
      Some(json!({
        "title": "Local Manifest Thread"
      })),
    ),
  );

  let turn_response = handle_request(
    &mut context,
    request(
      methods::TURN_START,
      Some(json!({
        "threadId": "thread-1",
        "message": "What version is in Cargo.toml?"
      })),
    ),
  );

  remove_temp_workspace(&workspace);

  assert!(turn_response.error.is_none());
  let result = turn_response.result.expect("turn result");
  let items = result["items"].as_array().expect("items");

  assert_eq!(items[2]["kind"], "toolStart");
  assert_eq!(items[2]["title"], "read_file");
  assert_eq!(items[2]["attributes"]["tool"], "read_file");
  assert_eq!(items[2]["attributes"]["relativePath"], "Cargo.toml");
  assert_eq!(items[3]["kind"], "toolResult");
  assert!(items[3]["content"].as_str().unwrap().contains("0.1.0"));
}

#[test]
fn turn_start_routes_fresh_find_requests_to_web_search_with_workspace_open() {
  let mut context = RuntimeContext::new_in_memory();
  enable_web_search_plugin(&mut context);
  let workspace = create_temp_workspace("fresh-web-with-workspace");
  fs::write(
    workspace.join("notes.txt"),
    "Local notes should not capture fresh public release lookup.\n",
  )
  .expect("write notes");

  let _ = handle_request(
    &mut context,
    request(
      methods::WORKSPACE_OPEN,
      Some(json!({
        "path": workspace.display().to_string()
      })),
    ),
  );
  let _ = handle_request(
    &mut context,
    request(
      methods::THREAD_START,
      Some(json!({
        "title": "Fresh Web Workspace Thread"
      })),
    ),
  );
  let _ = handle_request(
    &mut context,
    request(
      methods::TURN_CANCEL_RUNNING,
      Some(json!({
        "threadId": "thread-1"
      })),
    ),
  );

  let turn_response = handle_request(
    &mut context,
    request(
      methods::TURN_START,
      Some(json!({
        "threadId": "thread-1",
        "message": "Find latest Granite 4.0-H-350M release"
      })),
    ),
  );

  remove_temp_workspace(&workspace);

  assert!(turn_response.error.is_none());
  let result = turn_response.result.expect("turn result");
  let items = result["items"].as_array().expect("items");

  assert_eq!(items[1]["kind"], "plan");
  assert_eq!(items[1]["attributes"]["agentToolKind"], "web");
  assert_eq!(items[1]["attributes"]["agentToolName"], "web_search");
  assert_eq!(items[2]["kind"], "warning");
  assert_eq!(items[2]["title"], "Turn Cancelled");
}

#[test]
fn turn_start_executes_web_search_with_fixture_client() {
  let workspace = create_temp_workspace("web-search-fixture");
  let fixture_path = workspace.join("search.html");
  fs::write(
    &fixture_path,
    r#"
      <a rel="nofollow" href="//duckduckgo.com/l/?uddg=https%3A%2F%2Fexample.com%2Famentia&amp;rut=abc" class='result-link'>Amentia fixture result</a>
      <td class='result-snippet'>Deterministic local web search result.</td>
    "#,
  )
  .expect("write web search fixture");
  let _fixture_enabled_guard = EnvVarGuard::set(
    "AMENTIA_ENABLE_WEB_SEARCH_FIXTURE",
    std::ffi::OsStr::new("1"),
  );
  let _fixture_path_guard =
    EnvVarGuard::set("AMENTIA_WEB_SEARCH_FIXTURE_PATH", fixture_path.as_os_str());
  let mut context = RuntimeContext::new_in_memory();
  enable_web_search_plugin(&mut context);

  let _ = handle_request(
    &mut context,
    request(
      methods::THREAD_START,
      Some(json!({
        "title": "Fixture Web Thread"
      })),
    ),
  );

  let turn_response = handle_request(
    &mut context,
    request(
      methods::TURN_START,
      Some(json!({
        "threadId": "thread-1",
        "message": "web search for Amentia local model"
      })),
    ),
  );

  remove_temp_workspace(&workspace);

  assert!(turn_response.error.is_none());
  let result = turn_response.result.expect("turn result");
  let items = result["items"].as_array().expect("items");

  assert_eq!(items[2]["kind"], "toolStart");
  assert_eq!(items[2]["title"], "web_search");
  assert_eq!(items[2]["attributes"]["agentStepPhase"], "toolCall");
  assert_eq!(items[2]["attributes"]["agentLoopStepCount"], "1");
  assert_eq!(items[2]["attributes"]["agentToolKind"], "web");
  assert_eq!(items[2]["attributes"]["client"], "fixture");
  assert_eq!(items[3]["kind"], "toolResult");
  assert_eq!(items[3]["title"], "web_search result");
  assert_eq!(items[3]["attributes"]["agentStepPhase"], "observation");
  assert_eq!(items[3]["attributes"]["toolCallStatus"], "completed");
  assert_eq!(items[3]["attributes"]["resultCount"], "1");
  assert_eq!(
    items[3]["attributes"]["webSearchSourceMode"],
    "searchResultAttribution"
  );
  assert_eq!(items[3]["attributes"]["pageFetchPerformed"], "false");
  assert!(items[3]["content"]
    .as_str()
    .unwrap()
    .contains("Amentia fixture result"));
  assert_eq!(items[4]["kind"], "assistantMessage");
  assert_eq!(items[4]["attributes"]["responseRole"], "coworkHandoff");
  assert_eq!(items[4]["attributes"]["handoffKind"], "webSearchSources");
  assert_eq!(items[4]["attributes"]["sourceAttribution"], "web_search");
  assert_eq!(
    items[4]["attributes"]["webSearchSourceMode"],
    "searchResultAttribution"
  );
  assert_eq!(items[4]["attributes"]["pageFetchPerformed"], "false");
  assert_eq!(items[4]["attributes"]["sourceSnapshotAvailable"], "true");
  assert_eq!(
    items[4]["attributes"]["sourceSnapshotKind"],
    "searchResults"
  );
  assert_eq!(items[4]["attributes"]["sourceSnapshotResultCount"], "1");
  assert!(items[4]["attributes"]["sourceSnapshot"]
    .as_str()
    .unwrap()
    .contains("Deterministic local web search result."));
  assert_eq!(
    items[4]["attributes"]["sourceSnapshotHash"]
      .as_str()
      .unwrap()
      .len(),
    16
  );
  assert_eq!(
    items[4]["attributes"]["sourceUrls"],
    "https://example.com/amentia"
  );
  assert!(items[4]["attributes"]["sourceTitles"]
    .as_str()
    .unwrap()
    .contains("Amentia fixture result"));
}

struct EnvVarGuard {
  key: &'static str,
  previous: Option<OsString>,
}

impl EnvVarGuard {
  fn set(key: &'static str, value: &std::ffi::OsStr) -> Self {
    let previous = std::env::var_os(key);
    std::env::set_var(key, value);
    Self { key, previous }
  }
}

impl Drop for EnvVarGuard {
  fn drop(&mut self) {
    if let Some(previous) = self.previous.as_ref() {
      std::env::set_var(self.key, previous);
    } else {
      std::env::remove_var(self.key);
    }
  }
}

fn enable_web_search_plugin(context: &mut RuntimeContext) {
  replace_plugin_catalog(context, vec![web_search_plugin(true)]);
}

fn web_search_plugin(enabled: bool) -> amentia_plugin_host::PluginCatalogEntry {
  let default_enabled = true;
  bundled_plugin_entry(
    "web-search",
    "Web Search",
    enabled,
    default_enabled,
    &["tool:web_search"],
    &["network.outbound"],
  )
}
