use super::tests_support::manifest;
use super::validation::{manifest_capabilities, validate_manifest, validation_hint_for_error};
use super::*;

#[test]
fn validate_manifest_accepts_typed_capabilities_and_permissions() {
  let mut manifest = manifest(
    vec![
      "prompt_pack:workspace.notes",
      "settings:workspace.preferences",
    ],
    vec!["file.read", "file.write"],
  );
  manifest.skills = vec![PluginSkillManifest {
    id: "workspace.notes".to_string(),
    description: "Workspace note guidance.".to_string(),
    path: "skills/workspace-notes.md".to_string(),
  }];
  manifest.mcp_servers = vec![PluginMcpServerManifest {
    id: "workspace.mcp".to_string(),
    command: Some("pith-workspace-mcp".to_string()),
    args: vec![],
    transport: Some("stdio".to_string()),
  }];
  manifest.app_connectors = vec![PluginAppConnectorManifest {
    id: "workspace.connector".to_string(),
    display_name: "Workspace Connector".to_string(),
    service: "workspace".to_string(),
    homepage: None,
  }];
  manifest.auth_policy = Some(PluginAuthPolicyManifest {
    auth_type: "none".to_string(),
    required: false,
    scopes: vec![],
    credential_store: Some("local".to_string()),
  });

  let result = validate_manifest(&manifest);

  assert!(result.is_ok());
  let capabilities = manifest_capabilities(&manifest);
  assert!(capabilities
    .iter()
    .any(|capability| capability == "skill:workspace.notes"));
  assert!(capabilities
    .iter()
    .any(|capability| capability == "mcp_server:workspace.mcp"));
  assert!(capabilities
    .iter()
    .any(|capability| capability == "connector:workspace.connector"));
}

#[test]
fn validate_manifest_rejects_legacy_capability_format() {
  let manifest = manifest(vec!["memory.notes"], vec!["file.read"]);

  let error = validate_manifest(&manifest).expect_err("legacy capability should fail");

  assert!(error
    .to_string()
    .contains("must use the `<kind>:<identifier>` format"));
}

#[test]
fn validate_manifest_rejects_plugin_name_path_segments() {
  let mut manifest = manifest(vec!["command:review.focus"], vec!["file.read"]);
  manifest.name = "../focus-review".to_string();

  let error = validate_manifest(&manifest).expect_err("path segment plugin name should fail");

  assert!(error
    .to_string()
    .contains("must not contain path separators"));
}

#[test]
fn validate_manifest_rejects_capability_identifier_path_segments() {
  let manifest = manifest(vec!["command:../outside"], vec!["file.read"]);

  let error = validate_manifest(&manifest).expect_err("path segment capability should fail");

  assert!(error
    .to_string()
    .contains("must not contain path separators"));
}

#[test]
fn validation_hint_describes_supported_capability_kinds() {
  let hint = validation_hint_for_error("plugin capability kind `memory` is not supported");

  assert!(hint.contains("supported capability kinds"));
  assert!(hint.contains("command"));
  assert!(hint.contains("connector"));
  assert!(hint.contains("settings"));
}

#[test]
fn validation_hint_describes_supported_credential_stores() {
  let hint = validation_hint_for_error("plugin credential store `vault` is not supported");

  assert!(hint.contains("supported credential stores"));
  assert!(hint.contains("local"));
  assert!(!hint.contains("keychain"));
}
