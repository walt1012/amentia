use std::path::{Component, Path};

use anyhow::Result;

use crate::manifest::PluginManifest;

const KNOWN_CAPABILITY_KINDS: [&str; 10] = [
  "command",
  "agent",
  "prompt_pack",
  "hook",
  "tool",
  "mcp_server",
  "skill",
  "connector",
  "connector_workflow",
  "settings",
];
const KNOWN_PERMISSIONS: [&str; 7] = [
  "file.read",
  "file.write",
  "shell.exec",
  "network.outbound",
  "workspace.background",
  "model.invoke",
  "mcp.connect",
];
const KNOWN_AUTH_TYPES: [&str; 3] = ["none", "api_key", "oauth2"];
const KNOWN_CREDENTIAL_STORES: [&str; 2] = ["none", "local"];
const KNOWN_MCP_TRANSPORTS: [&str; 1] = ["stdio"];
const MIN_CONNECTOR_WORKFLOW_AGENT_STEPS: usize = 1;
const MAX_CONNECTOR_WORKFLOW_AGENT_STEPS: usize = 8;

pub(crate) fn validation_hint_for_error(validation_error: &str) -> String {
  if validation_error.contains("failed to parse plugin manifest") {
    return "Check that amentia-plugin.json is valid JSON and uses camelCase field names such as `displayName` and `defaultEnabled`."
      .to_string();
  }

  if validation_error.contains("must use the `<kind>:<identifier>` format") {
    return format!(
      "Rewrite each capability as `<kind>:<identifier>`. Supported kinds: {}.",
      KNOWN_CAPABILITY_KINDS.join(", ")
    );
  }

  if validation_error.contains("capability kind") && validation_error.contains("is not supported") {
    return format!(
      "Use one of the supported capability kinds: {}.",
      KNOWN_CAPABILITY_KINDS.join(", ")
    );
  }

  if validation_error.contains("must include a non-empty identifier") {
    return "Add a non-empty identifier after the capability kind, for example `command:workspace.capture-note`."
      .to_string();
  }

  if validation_error.contains("relative path segment")
    || validation_error.contains("path separators")
    || validation_error.contains("must not contain `:`")
    || validation_error.contains("must not contain whitespace")
  {
    return "Use stable plugin identifiers without spaces, colons, or path separators, for example `notion-connector`."
      .to_string();
  }

  if validation_error.contains("plugin permission") && validation_error.contains("is not supported")
  {
    return format!(
      "Use one of the supported permissions: {}.",
      KNOWN_PERMISSIONS.join(", ")
    );
  }
  if validation_error.contains("path must stay inside the plugin bundle") {
    return "Use a normal relative path inside the plugin bundle, for example `skills/workspace-notes.md`."
      .to_string();
  }
  if validation_error.contains("plugin auth policy type")
    && validation_error.contains("is not supported")
  {
    return format!(
      "Use one of the supported auth policy types: {}.",
      KNOWN_AUTH_TYPES.join(", ")
    );
  }
  if validation_error.contains("plugin auth policy type `none` must not require credentials") {
    return "Set auth policy `required` to false, or switch to `api_key` or `oauth2` with `credentialStore: local`."
      .to_string();
  }
  if validation_error.contains("plugin auth policy type `none` must not declare scopes") {
    return "Remove auth scopes for `none`, or switch to an authenticated policy with `credentialStore: local`."
      .to_string();
  }
  if validation_error.contains("plugin auth policy type `none` must use credential store `none`") {
    return "Use `credentialStore: none` for auth-free connectors, or switch to an authenticated policy with `credentialStore: local`."
      .to_string();
  }
  if validation_error.contains("must include a non-empty display name") {
    return "Add a human-readable displayName so Amentia can show the integration clearly."
      .to_string();
  }
  if validation_error.contains("must include a non-empty service") {
    return "Add a stable service such as `notion` or another lowercase service key.".to_string();
  }
  if validation_error.contains("plugin connector workflow")
    && validation_error.contains("references undeclared connector")
  {
    return "Declare the connector in appConnectors before attaching connectorWorkflows to it."
      .to_string();
  }
  if validation_error.contains("plugin connector workflow capability")
    && validation_error.contains("connectorWorkflows entry")
  {
    return "Add a matching connectorWorkflows entry or remove the unused connector_workflow capability."
      .to_string();
  }
  if validation_error.contains("plugin prompt pack capability")
    && validation_error.contains("skill entry")
  {
    let skill_capability = validation_error
      .split('`')
      .nth(1)
      .and_then(|capability| capability.strip_prefix("prompt_pack:"))
      .map(|identifier| format!("skill:{identifier}"))
      .unwrap_or_else(|| "skill:<id>".to_string());
    return format!(
      "Add a matching skills entry and {skill_capability} capability, or remove the legacy prompt_pack alias."
    );
  }
  if validation_error.contains("plugin prompt pack capability")
    && validation_error.contains("matching capability")
  {
    return "Add the matching skill capability shown in the error, or remove the legacy prompt_pack alias."
      .to_string();
  }
  if validation_error.contains("plugin connector workflow")
    && validation_error.contains("must declare at least one")
  {
    return "Declare the connector workflow stages and statuses that runner output is allowed to report."
      .to_string();
  }
  if validation_error.contains("plugin connector workflow")
    && validation_error.contains("maxAgentSteps")
  {
    return "Set maxAgentSteps to a bounded value between 1 and 8, or omit it to use the default loop budget."
      .to_string();
  }
  if validation_error.contains("plugin credential store")
    && validation_error.contains("is not supported")
  {
    return format!(
      "Use one of the supported credential stores: {}.",
      KNOWN_CREDENTIAL_STORES.join(", ")
    );
  }
  if validation_error.contains("plugin auth policy credential store is required") {
    return "Declare `credentialStore: local` for authenticated connectors, or use auth policy type `none` with credential store `none`."
      .to_string();
  }
  if validation_error.contains("plugin MCP server transport")
    && validation_error.contains("is not supported")
  {
    return format!(
      "Use one of the supported MCP transports: {}.",
      KNOWN_MCP_TRANSPORTS.join(", ")
    );
  }

  "Review the manifest schema, fix the reported field or value, then refresh plugins.".to_string()
}

pub(crate) fn validate_manifest(manifest: &PluginManifest) -> Result<()> {
  validate_manifest_identifier("name", &manifest.name)?;

  for capability in manifest_capabilities(manifest) {
    let Some((kind, identifier)) = capability.split_once(':') else {
      anyhow::bail!(
        "plugin capability `{}` must use the `<kind>:<identifier>` format",
        capability
      );
    };
    if !KNOWN_CAPABILITY_KINDS.contains(&kind) {
      anyhow::bail!("plugin capability kind `{}` is not supported", kind);
    }
    if identifier.trim().is_empty() {
      anyhow::bail!(
        "plugin capability `{}` must include a non-empty identifier",
        capability
      );
    }
    validate_manifest_identifier("capability", identifier)?;
  }

  for skill in &manifest.skills {
    validate_manifest_identifier("skill", &skill.id)?;
    if skill.path.trim().is_empty() {
      anyhow::bail!("plugin skill `{}` must include a non-empty path", skill.id);
    }
    validate_bundle_relative_path(&skill.path).map_err(|_| {
      anyhow::anyhow!(
        "plugin skill `{}` path must stay inside the plugin bundle",
        skill.id
      )
    })?;
  }

  for server in &manifest.mcp_servers {
    validate_manifest_identifier("mcp server", &server.id)?;
    if let Some(transport) = server.transport.as_ref() {
      if !KNOWN_MCP_TRANSPORTS.contains(&transport.as_str()) {
        anyhow::bail!(
          "plugin MCP server transport `{}` is not supported",
          transport
        );
      }
    }
    if let Some(command) = server.command.as_ref() {
      if command.trim().is_empty() {
        anyhow::bail!(
          "plugin MCP server `{}` command must not be empty",
          server.id
        );
      }
      validate_bundle_relative_path(command).map_err(|_| {
        anyhow::anyhow!(
          "plugin MCP server `{}` command must stay inside the plugin bundle",
          server.id
        )
      })?;
    }
  }

  for connector in &manifest.app_connectors {
    validate_manifest_identifier("connector", &connector.id)?;
    if connector.display_name.trim().is_empty() {
      anyhow::bail!(
        "plugin connector `{}` must include a non-empty display name",
        connector.id
      );
    }
    if connector.service.trim().is_empty() {
      anyhow::bail!(
        "plugin connector `{}` must include a non-empty service",
        connector.id
      );
    }
  }

  for workflow in &manifest.connector_workflows {
    validate_manifest_identifier("connector workflow", &workflow.id)?;
    validate_manifest_identifier("connector workflow connector", &workflow.connector_id)?;
    validate_manifest_identifier("connector workflow action", &workflow.action)?;
    if workflow.display_name.trim().is_empty() {
      anyhow::bail!(
        "plugin connector workflow `{}` must include a non-empty display name",
        workflow.id
      );
    }
    if !manifest
      .app_connectors
      .iter()
      .any(|connector| connector.id == workflow.connector_id)
    {
      anyhow::bail!(
        "plugin connector workflow `{}` references undeclared connector `{}`",
        workflow.id,
        workflow.connector_id
      );
    }
    if workflow.stages.is_empty() {
      anyhow::bail!(
        "plugin connector workflow `{}` must declare at least one stage",
        workflow.id
      );
    }
    if workflow.statuses.is_empty() {
      anyhow::bail!(
        "plugin connector workflow `{}` must declare at least one status",
        workflow.id
      );
    }
    if let Some(max_agent_steps) = workflow.max_agent_steps {
      if !(MIN_CONNECTOR_WORKFLOW_AGENT_STEPS..=MAX_CONNECTOR_WORKFLOW_AGENT_STEPS)
        .contains(&max_agent_steps)
      {
        anyhow::bail!(
          "plugin connector workflow `{}` maxAgentSteps must be between {} and {}",
          workflow.id,
          MIN_CONNECTOR_WORKFLOW_AGENT_STEPS,
          MAX_CONNECTOR_WORKFLOW_AGENT_STEPS
        );
      }
    }
    for stage in &workflow.stages {
      validate_manifest_identifier("connector workflow stage", stage)?;
    }
    for status in &workflow.statuses {
      validate_manifest_identifier("connector workflow status", status)?;
    }
  }
  for capability in &manifest.capabilities {
    let Some(("connector_workflow", workflow_id)) = capability.split_once(':') else {
      continue;
    };
    if !manifest
      .connector_workflows
      .iter()
      .any(|workflow| workflow.id == workflow_id)
    {
      anyhow::bail!(
        "plugin connector workflow capability `{}` must match a connectorWorkflows entry",
        capability
      );
    }
  }
  for capability in &manifest.capabilities {
    let Some(("prompt_pack", prompt_pack_id)) = capability.split_once(':') else {
      continue;
    };
    if !manifest.skills.iter().any(|skill| skill.id == prompt_pack_id) {
      anyhow::bail!(
        "plugin prompt pack capability `{}` must map to a skill entry",
        capability
      );
    }
    let skill_capability = format!("skill:{prompt_pack_id}");
    if !manifest
      .capabilities
      .iter()
      .any(|capability| capability == &skill_capability)
    {
      anyhow::bail!(
        "plugin prompt pack capability `{}` must include matching capability `{}`",
        capability,
        skill_capability
      );
    }
  }

  if let Some(auth_policy) = manifest.auth_policy.as_ref() {
    if !KNOWN_AUTH_TYPES.contains(&auth_policy.auth_type.as_str()) {
      anyhow::bail!(
        "plugin auth policy type `{}` is not supported",
        auth_policy.auth_type
      );
    }
    if auth_policy.auth_type == "none" {
      if auth_policy.required {
        anyhow::bail!("plugin auth policy type `none` must not require credentials");
      }
      if !auth_policy.scopes.is_empty() {
        anyhow::bail!("plugin auth policy type `none` must not declare scopes");
      }
      if let Some(credential_store) = auth_policy.credential_store.as_ref() {
        if credential_store != "none" {
          anyhow::bail!("plugin auth policy type `none` must use credential store `none`");
        }
      }
    } else {
      match auth_policy.credential_store.as_deref() {
        Some("local") => {}
        Some("none") => {
          anyhow::bail!(
            "plugin auth policy credential store is required for authenticated connectors"
          );
        }
        None => {
          anyhow::bail!(
            "plugin auth policy credential store is required for authenticated connectors"
          );
        }
        Some(_) => {}
      }
    }
    if let Some(credential_store) = auth_policy.credential_store.as_ref() {
      if !KNOWN_CREDENTIAL_STORES.contains(&credential_store.as_str()) {
        anyhow::bail!(
          "plugin credential store `{}` is not supported",
          credential_store
        );
      }
    }
  }

  for permission in &manifest.permissions {
    if !KNOWN_PERMISSIONS.contains(&permission.as_str()) {
      anyhow::bail!("plugin permission `{}` is not supported", permission);
    }
  }

  Ok(())
}

pub(crate) fn manifest_capabilities(manifest: &PluginManifest) -> Vec<String> {
  let mut capabilities = manifest.capabilities.clone();
  for skill in &manifest.skills {
    push_unique_capability(&mut capabilities, "skill", &skill.id);
  }
  for server in &manifest.mcp_servers {
    push_unique_capability(&mut capabilities, "mcp_server", &server.id);
  }
  for connector in &manifest.app_connectors {
    push_unique_capability(&mut capabilities, "connector", &connector.id);
  }
  for workflow in &manifest.connector_workflows {
    push_unique_capability(&mut capabilities, "connector_workflow", &workflow.id);
  }
  capabilities
}

fn validate_manifest_identifier(kind: &str, identifier: &str) -> Result<()> {
  if identifier.trim().is_empty() {
    anyhow::bail!("plugin {kind} identifier must not be empty");
  }
  if identifier == "." || identifier == ".." {
    anyhow::bail!("plugin {kind} identifier `{identifier}` must not be a relative path segment");
  }
  if identifier.contains('/') || identifier.contains('\\') {
    anyhow::bail!("plugin {kind} identifier `{identifier}` must not contain path separators");
  }
  if identifier.contains(':') {
    anyhow::bail!("plugin {kind} identifier `{identifier}` must not contain `:`");
  }
  if identifier.chars().any(char::is_whitespace) {
    anyhow::bail!("plugin {kind} identifier `{identifier}` must not contain whitespace");
  }
  Ok(())
}

fn validate_bundle_relative_path(path: &str) -> Result<()> {
  let trimmed = path.trim();
  if trimmed.is_empty()
    || trimmed.contains('\\')
    || trimmed.contains(':')
    || Path::new(trimmed).is_absolute()
  {
    anyhow::bail!("path must stay inside the plugin bundle");
  }
  for component in Path::new(trimmed).components() {
    match component {
      Component::Normal(_) => {}
      _ => anyhow::bail!("path must stay inside the plugin bundle"),
    }
  }
  Ok(())
}

fn push_unique_capability(capabilities: &mut Vec<String>, kind: &str, identifier: &str) {
  let capability = format!("{kind}:{identifier}");
  if !capabilities.iter().any(|existing| existing == &capability) {
    capabilities.push(capability);
  }
}
