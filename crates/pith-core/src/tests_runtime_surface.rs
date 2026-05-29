use super::test_support::{bundled_plugin_entry, replace_plugin_catalog, request};
use super::*;
use pith_protocol::methods;
use serde_json::json;

#[test]
fn initialize_request_returns_capabilities() {
  let mut context = RuntimeContext::new_in_memory();
  let response = handle_request(
    &mut context,
    request(
      methods::INITIALIZE,
      Some(json!({
        "clientInfo": {
          "name": "pith-tests",
          "version": "0.1.0"
        }
      })),
    ),
  );

  assert!(response.error.is_none());
  let result = response.result.expect("initialize result");
  assert_eq!(result["protocolVersion"], "0.1.0");
  assert_eq!(result["capabilities"]["supportsRuntimeReadiness"], true);
  assert_eq!(result["capabilities"]["supportsThreads"], true);
  assert_eq!(result["capabilities"]["supportsTools"], true);
}

#[test]
fn health_ping_returns_ok() {
  let mut context = RuntimeContext::new_in_memory();
  let response = handle_request(&mut context, request(methods::HEALTH_PING, None));

  assert!(response.error.is_none());
  let result = response.result.expect("health result");
  assert_eq!(result["status"], "ok");
}

#[test]
fn runtime_readiness_reports_agent_control_surface() {
  let mut context = RuntimeContext::new_in_memory();
  let response = handle_request(&mut context, request(methods::RUNTIME_READINESS, None));

  assert!(response.error.is_none());
  let result = response.result.expect("runtime readiness result");
  assert_eq!(result["status"], "setup_required");
  assert!(result["summary"]
    .as_str()
    .expect("summary")
    .contains("local agent work"));
  let checks = result["checks"].as_array().expect("checks");
  let check_ids = checks
    .iter()
    .filter_map(|check| check["id"].as_str())
    .collect::<Vec<_>>();
  assert!(check_ids.contains(&"localModel"));
  assert!(check_ids.contains(&"workspace"));
  assert!(check_ids.contains(&"thread"));
  assert!(check_ids.contains(&"firstRequest"));
  assert!(check_ids.contains(&"nativeSandbox"));
  assert!(check_ids.contains(&"webSearch"));
  assert!(check_ids.contains(&"boundedRuntime"));
  assert_eq!(result["metrics"]["sandboxMode"], "workspaceReadWrite");
  assert!(result["metrics"]["sandboxBackend"].is_string());
  assert_eq!(result["metrics"]["sandboxNetworkAllowed"], "false");
  assert_eq!(
    result["metrics"]["sandboxNetworkPolicy"],
    "network denied by policy, not native-enforced"
  );
  assert_eq!(result["metrics"]["contextWindowTokens"], "4096");
  assert_eq!(result["metrics"]["dailyDriverStage"], "model_setup");
  assert_eq!(
    result["metrics"]["dailyDriverNextAction"],
    "Download and select a verified local model."
  );
  assert_eq!(result["metrics"]["pithAccountRequired"], "false");
  assert_eq!(
    result["metrics"]["defaultLocalExecutionSafetyMode"],
    "askBeforeChange"
  );
  assert_eq!(
    result["metrics"]["localExecutionSafetyModes"],
    "explore,askBeforeChange,approvedWorkspaceExecution"
  );
  assert_eq!(result["metrics"]["workspaceThreadCount"], "0");
  assert_eq!(result["metrics"]["firstRequestSent"], "false");
  assert_eq!(result["metrics"]["activeTurnCount"], "0");
  assert_eq!(result["metrics"]["runningTurnCount"], "0");
  assert_eq!(result["metrics"]["runningApprovalCount"], "0");
  assert_eq!(result["metrics"]["runningPluginCommandCount"], "0");
  assert_eq!(result["metrics"]["runningWorkspaceSearchCount"], "0");
  assert!(result["metrics"]["shellOutputArtifactRoot"].is_string());
  assert_eq!(result["metrics"]["shellOutputArtifactRetainedRuns"], "20");
  assert_eq!(result["metrics"]["workspaceSearchMaxFileBytes"], "262144");
  assert_eq!(
    result["metrics"]["workspaceSearchMaxVisitedEntries"],
    "20000"
  );
  assert_eq!(
    result["metrics"]["directoryListingMaxScannedEntries"],
    "5000"
  );
  assert_eq!(result["metrics"]["diffPreviewMaxBytes"], "131072");
  assert_eq!(result["metrics"]["workspaceWriteMaxBytes"], "1048576");
  assert_eq!(result["metrics"]["turnReadFileMaxBytes"], "4096");
  assert_eq!(result["metrics"]["turnListDirectoryMaxResults"], "24");
  assert_eq!(result["metrics"]["turnSearchFilesMaxResults"], "12");
  assert_eq!(result["metrics"]["turnShellOutputMaxBytes"], "4096");
  assert_eq!(result["metrics"]["turnWebSearchMaxResults"], "5");
  assert_eq!(result["metrics"]["webSearchTimeoutSeconds"], "20");
  assert_eq!(result["metrics"]["webSearchProvider"], "DuckDuckGo Lite");
  assert!(matches!(
    result["metrics"]["webSearchClient"].as_str(),
    Some("curl" | "fixture")
  ));
  assert!(result["metrics"]["webSearchAvailable"].is_string());
  assert!(result["metrics"]["webSearchPermissionGranted"].is_string());
  assert!(result["metrics"]["webSearchPermissionSources"].is_string());
  assert!(result["metrics"]["pluginRootCount"].is_string());
  assert!(result["metrics"]["pluginRoots"].is_string());
  assert!(result["metrics"]["pluginInstallRoot"].is_string());
  let local_model = checks
    .iter()
    .find(|check| check["id"] == "localModel")
    .expect("local model check");
  if local_model["status"].as_str() != Some("ready") {
    assert!(local_model["detail"]
      .as_str()
      .expect("local model detail")
      .contains("Local model runtime is unavailable"));
  }
}

#[test]
fn runtime_readiness_web_search_requires_enabled_network_plugin() {
  let mut context = RuntimeContext::new_in_memory();
  replace_plugin_catalog(
    &mut context,
    vec![bundled_plugin_entry(
      "web-search",
      "Web Search",
      false,
      true,
      &["tool:web_search"],
      &["network.outbound"],
    )],
  );
  let response = handle_request(&mut context, request(methods::RUNTIME_READINESS, None));

  assert!(response.error.is_none());
  let result = response.result.expect("runtime readiness result");
  let web_search = result["checks"]
    .as_array()
    .expect("checks")
    .iter()
    .find(|check| check["id"] == "webSearch")
    .expect("web search check");
  assert_eq!(web_search["status"], "setup_required");
  assert!(web_search["detail"]
    .as_str()
    .expect("detail")
    .contains("Enable the Web Search plugin"));
  assert_eq!(result["metrics"]["webSearchPermissionGranted"], "false");
  assert_eq!(result["metrics"]["webSearchPermissionSources"], "");
}

#[test]
fn model_health_returns_local_model_status() {
  let mut context = RuntimeContext::new_in_memory();
  let response = handle_request(&mut context, request(methods::MODEL_HEALTH, None));

  assert!(response.error.is_none());
  let result = response.result.expect("model health result");
  assert_eq!(result["displayName"], "LFM2.5-350M");
  assert!(result["backend"].is_string());
  assert!(result["status"].is_string());
}

#[test]
fn turn_start_requires_ready_model_when_runtime_enforces_readiness() {
  let mut context = RuntimeContext::new_in_memory();
  context.model_state.set_enforce_readiness(true);
  let response = handle_request(
    &mut context,
    request(
      methods::TURN_START,
      Some(json!({
        "threadId": "local-welcome",
        "message": "Inspect the workspace"
      })),
    ),
  );

  let error = response.error.expect("model readiness error");
  assert_eq!(error.code, -32060);
  assert!(error.message.contains("Local model is not ready"));
}

#[test]
fn unknown_method_returns_json_rpc_error() {
  let mut context = RuntimeContext::new_in_memory();
  let response = handle_request(&mut context, request("unknown/method", None));

  assert!(response.result.is_none());
  let error = response.error.expect("error payload");
  assert_eq!(error.code, -32601);
}
