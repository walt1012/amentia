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
    command: Some("amentia-workspace-mcp".to_string()),
    args: vec![],
    transport: Some("stdio".to_string()),
  }];
  manifest.app_connectors = vec![PluginAppConnectorManifest {
    id: "workspace.connector".to_string(),
    display_name: "Workspace Connector".to_string(),
    service: "workspace".to_string(),
    homepage: None,
  }];
  manifest.connector_workflows = vec![PluginConnectorWorkflowManifest {
    id: "workspace.publish".to_string(),
    display_name: "Workspace Publish".to_string(),
    connector_id: "workspace.connector".to_string(),
    action: "publishPage".to_string(),
    max_agent_steps: Some(4),
    stages: vec!["draft".to_string(), "published".to_string()],
    statuses: vec!["prepared".to_string(), "completed".to_string()],
  }];
  manifest.auth_policy = Some(PluginAuthPolicyManifest {
    auth_type: "none".to_string(),
    required: false,
    scopes: vec![],
    credential_store: Some("none".to_string()),
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
  assert!(capabilities
    .iter()
    .any(|capability| capability == "connector_workflow:workspace.publish"));
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
fn validate_manifest_rejects_plugin_name_whitespace() {
  let mut manifest = manifest(vec!["command:review.focus"], vec!["file.read"]);
  manifest.name = "focus review".to_string();

  let error = validate_manifest(&manifest).expect_err("whitespace plugin name should fail");

  assert!(error.to_string().contains("must not contain whitespace"));
}

#[test]
fn validate_manifest_rejects_plugin_name_colons() {
  let mut manifest = manifest(vec!["command:review.focus"], vec!["file.read"]);
  manifest.name = "focus:review".to_string();

  let error = validate_manifest(&manifest).expect_err("colon plugin name should fail");

  assert!(error.to_string().contains("must not contain `:`"));
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
fn validate_manifest_rejects_capability_identifier_whitespace() {
  let manifest = manifest(vec!["command:review focus"], vec!["file.read"]);

  let error = validate_manifest(&manifest).expect_err("whitespace capability should fail");

  assert!(error.to_string().contains("must not contain whitespace"));
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
fn validation_hint_describes_stable_identifiers_for_whitespace() {
  let hint =
    validation_hint_for_error("plugin name identifier `focus review` must not contain whitespace");

  assert!(hint.contains("stable plugin identifiers"));
  assert!(hint.contains("spaces"));
  assert!(hint.contains("colons"));
  assert!(hint.contains("notion-connector"));
}

#[test]
fn validation_hint_describes_stable_identifiers_for_colons() {
  let hint =
    validation_hint_for_error("plugin name identifier `focus:review` must not contain `:`");

  assert!(hint.contains("stable plugin identifiers"));
  assert!(hint.contains("colons"));
  assert!(hint.contains("notion-connector"));
}

#[test]
fn validation_hint_describes_supported_credential_stores() {
  let hint = validation_hint_for_error("plugin credential store `vault` is not supported");

  assert!(hint.contains("supported credential stores"));
  assert!(hint.contains("local"));
  assert!(!hint.contains("keychain"));
}

#[test]
fn validation_hint_describes_supported_auth_policy_types() {
  let hint = validation_hint_for_error("plugin auth policy type `saml` is not supported");

  assert!(hint.contains("supported auth policy types"));
  assert!(hint.contains("api_key"));
  assert!(hint.contains("oauth2"));
}

#[test]
fn validation_hint_explains_auth_free_policy_requirements() {
  let required_hint =
    validation_hint_for_error("plugin auth policy type `none` must not require credentials");
  let scopes_hint =
    validation_hint_for_error("plugin auth policy type `none` must not declare scopes");
  let store_hint =
    validation_hint_for_error("plugin auth policy type `none` must use credential store `none`");

  assert!(required_hint.contains("required"));
  assert!(required_hint.contains("credentialStore: local"));
  assert!(scopes_hint.contains("Remove auth scopes"));
  assert!(store_hint.contains("credentialStore: none"));
}

#[test]
fn validation_hint_explains_integration_identity_fields() {
  let display_name_hint =
    validation_hint_for_error("plugin connector `notion` must include a non-empty display name");
  let service_hint =
    validation_hint_for_error("plugin connector `notion` must include a non-empty service");

  assert!(display_name_hint.contains("displayName"));
  assert!(service_hint.contains("service"));
  assert!(service_hint.contains("notion"));
}

#[test]
fn validation_hint_describes_required_authenticated_credential_store() {
  let hint = validation_hint_for_error(
    "plugin auth policy credential store is required for authenticated connectors",
  );

  assert!(hint.contains("credentialStore: local"));
  assert!(hint.contains("authenticated connectors"));
}

#[test]
fn validation_hint_describes_connector_workflow_repair() {
  let connector_hint = validation_hint_for_error(
    "plugin connector workflow `notion.create-page` references undeclared connector `notion`",
  );
  let stage_hint = validation_hint_for_error(
    "plugin connector workflow `notion.create-page` must declare at least one stage",
  );

  assert!(connector_hint.contains("appConnectors"));
  assert!(stage_hint.contains("stages"));
  assert!(stage_hint.contains("statuses"));
  let capability_hint = validation_hint_for_error(
    "plugin connector workflow capability `connector_workflow:notion.create-page` must match a connectorWorkflows entry",
  );
  assert!(capability_hint.contains("connectorWorkflows"));
}

#[test]
fn validation_hint_describes_supported_mcp_transports() {
  let hint = validation_hint_for_error("plugin MCP server transport `http` is not supported");

  assert!(hint.contains("supported MCP transports"));
  assert!(hint.contains("stdio"));
}

#[test]
fn validation_hint_uses_refresh_action_for_unknown_manifest_errors() {
  let hint = validation_hint_for_error("plugin settings field is not supported yet");

  assert!(hint.contains("refresh plugins"));
}

#[test]
fn validate_manifest_rejects_unsupported_mcp_transport() {
  let mut manifest = manifest(vec!["mcp_server:notion"], vec!["mcp.connect"]);
  manifest.mcp_servers = vec![PluginMcpServerManifest {
    id: "notion".to_string(),
    command: Some("https://example.invalid/mcp".to_string()),
    args: vec![],
    transport: Some("http".to_string()),
  }];

  let error = validate_manifest(&manifest).expect_err("remote MCP is not supported yet");

  assert!(error
    .to_string()
    .contains("plugin MCP server transport `http` is not supported"));
}

#[test]
fn validate_manifest_rejects_unimplemented_keychain_store() {
  let mut manifest = manifest(vec!["connector:notion"], vec!["network.outbound"]);
  manifest.app_connectors = vec![PluginAppConnectorManifest {
    id: "notion".to_string(),
    display_name: "Notion".to_string(),
    service: "notion".to_string(),
    homepage: None,
  }];
  manifest.auth_policy = Some(PluginAuthPolicyManifest {
    auth_type: "oauth2".to_string(),
    required: true,
    scopes: vec!["read_content".to_string()],
    credential_store: Some("keychain".to_string()),
  });

  let error = validate_manifest(&manifest).expect_err("keychain is not implemented yet");

  assert!(error
    .to_string()
    .contains("plugin credential store `keychain` is not supported"));
}

#[test]
fn validate_manifest_rejects_unsupported_auth_policy_type() {
  let mut manifest = manifest(vec!["connector:notion"], vec!["network.outbound"]);
  manifest.app_connectors = vec![PluginAppConnectorManifest {
    id: "notion".to_string(),
    display_name: "Notion".to_string(),
    service: "notion".to_string(),
    homepage: None,
  }];
  manifest.auth_policy = Some(PluginAuthPolicyManifest {
    auth_type: "saml".to_string(),
    required: true,
    scopes: vec!["read_content".to_string()],
    credential_store: Some("local".to_string()),
  });

  let error = validate_manifest(&manifest).expect_err("unsupported auth type should fail");

  assert!(error
    .to_string()
    .contains("plugin auth policy type `saml` is not supported"));
}

#[test]
fn validate_manifest_rejects_connector_without_display_name() {
  let mut manifest = manifest(vec!["connector:notion"], vec!["network.outbound"]);
  manifest.app_connectors = vec![PluginAppConnectorManifest {
    id: "notion".to_string(),
    display_name: " ".to_string(),
    service: "notion".to_string(),
    homepage: None,
  }];

  let error = validate_manifest(&manifest).expect_err("connector display name should be required");

  assert!(error
    .to_string()
    .contains("plugin connector `notion` must include a non-empty display name"));
}

#[test]
fn validate_manifest_requires_local_store_for_authenticated_policy() {
  let mut manifest = manifest(vec!["connector:notion"], vec!["network.outbound"]);
  manifest.app_connectors = vec![PluginAppConnectorManifest {
    id: "notion".to_string(),
    display_name: "Notion".to_string(),
    service: "notion".to_string(),
    homepage: None,
  }];
  manifest.auth_policy = Some(PluginAuthPolicyManifest {
    auth_type: "oauth2".to_string(),
    required: true,
    scopes: vec!["read_content".to_string()],
    credential_store: None,
  });

  let error = validate_manifest(&manifest).expect_err("authenticated connectors need a store");

  assert!(error
    .to_string()
    .contains("plugin auth policy credential store is required"));
}

#[test]
fn validate_manifest_rejects_none_store_for_authenticated_policy() {
  let mut manifest = manifest(vec!["connector:notion"], vec!["network.outbound"]);
  manifest.app_connectors = vec![PluginAppConnectorManifest {
    id: "notion".to_string(),
    display_name: "Notion".to_string(),
    service: "notion".to_string(),
    homepage: None,
  }];
  manifest.auth_policy = Some(PluginAuthPolicyManifest {
    auth_type: "api_key".to_string(),
    required: true,
    scopes: vec![],
    credential_store: Some("none".to_string()),
  });

  let error = validate_manifest(&manifest).expect_err("authenticated connectors need local store");

  assert!(error
    .to_string()
    .contains("plugin auth policy credential store is required"));
}

#[test]
fn validate_manifest_rejects_local_store_for_auth_free_policy() {
  let mut manifest = manifest(vec!["connector:web"], vec!["network.outbound"]);
  manifest.app_connectors = vec![PluginAppConnectorManifest {
    id: "web".to_string(),
    display_name: "Web".to_string(),
    service: "web".to_string(),
    homepage: None,
  }];
  manifest.auth_policy = Some(PluginAuthPolicyManifest {
    auth_type: "none".to_string(),
    required: false,
    scopes: vec![],
    credential_store: Some("local".to_string()),
  });

  let error = validate_manifest(&manifest).expect_err("auth-free connectors use no credential");

  assert!(error
    .to_string()
    .contains("plugin auth policy type `none` must use credential store `none`"));
}

#[test]
fn validate_manifest_rejects_connector_workflow_without_connector() {
  let mut manifest = manifest(
    vec!["connector_workflow:notion.create-page"],
    vec!["network.outbound"],
  );
  manifest.connector_workflows = vec![PluginConnectorWorkflowManifest {
    id: "notion.create-page".to_string(),
    display_name: "Notion Create Page".to_string(),
    connector_id: "notion".to_string(),
    action: "createPage".to_string(),
    max_agent_steps: None,
    stages: vec!["draftPrepared".to_string()],
    statuses: vec!["prepared".to_string()],
  }];

  let error = validate_manifest(&manifest).expect_err("workflow needs a connector");

  assert!(error
    .to_string()
    .contains("references undeclared connector `notion`"));
}

#[test]
fn validate_manifest_rejects_connector_workflow_capability_without_contract() {
  let manifest = manifest(
    vec!["connector:notion", "connector_workflow:notion.create-page"],
    vec!["network.outbound"],
  );

  let error = validate_manifest(&manifest).expect_err("workflow capability needs contract");

  assert!(error
    .to_string()
    .contains("must match a connectorWorkflows entry"));
}

#[test]
fn validate_manifest_rejects_connector_workflow_without_lifecycle_shape() {
  let mut manifest = manifest(
    vec!["connector:notion", "connector_workflow:notion.create-page"],
    vec!["network.outbound"],
  );
  manifest.app_connectors = vec![PluginAppConnectorManifest {
    id: "notion".to_string(),
    display_name: "Notion".to_string(),
    service: "notion".to_string(),
    homepage: None,
  }];
  manifest.connector_workflows = vec![PluginConnectorWorkflowManifest {
    id: "notion.create-page".to_string(),
    display_name: "Notion Create Page".to_string(),
    connector_id: "notion".to_string(),
    action: "createPage".to_string(),
    max_agent_steps: None,
    stages: vec![],
    statuses: vec!["prepared".to_string()],
  }];

  let error = validate_manifest(&manifest).expect_err("workflow needs stages");

  assert!(error
    .to_string()
    .contains("must declare at least one stage"));
}

#[test]
fn validate_manifest_rejects_unbounded_connector_workflow_steps() {
  let mut manifest = manifest(
    vec!["connector:notion", "connector_workflow:notion.create-page"],
    vec!["network.outbound"],
  );
  manifest.app_connectors = vec![PluginAppConnectorManifest {
    id: "notion".to_string(),
    display_name: "Notion".to_string(),
    service: "notion".to_string(),
    homepage: None,
  }];
  manifest.connector_workflows = vec![PluginConnectorWorkflowManifest {
    id: "notion.create-page".to_string(),
    display_name: "Notion Create Page".to_string(),
    connector_id: "notion".to_string(),
    action: "createPage".to_string(),
    max_agent_steps: Some(9),
    stages: vec!["draftPrepared".to_string()],
    statuses: vec!["prepared".to_string()],
  }];

  let error = validate_manifest(&manifest).expect_err("workflow step budget should be bounded");

  assert!(error.to_string().contains("maxAgentSteps"));
}
