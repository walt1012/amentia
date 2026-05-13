use anyhow::Result;

use crate::manifest::PluginManifest;

const KNOWN_CAPABILITY_KINDS: [&str; 9] = [
  "command",
  "agent",
  "prompt_pack",
  "hook",
  "tool",
  "mcp_server",
  "skill",
  "connector",
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
const KNOWN_CREDENTIAL_STORES: [&str; 2] = ["none", "keychain"];

pub(crate) fn validation_hint_for_error(validation_error: &str) -> String {
  if validation_error.contains("failed to parse plugin manifest") {
    return "Check that pith-plugin.json is valid JSON and uses camelCase field names such as `displayName` and `defaultEnabled`."
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
  {
    return "Use stable plugin identifiers without path separators, for example `notion-connector`."
      .to_string();
  }

  if validation_error.contains("plugin permission") && validation_error.contains("is not supported")
  {
    return format!(
      "Use one of the supported permissions: {}.",
      KNOWN_PERMISSIONS.join(", ")
    );
  }

  "Review the manifest schema, then fix the reported field or value and reload the plugin catalog."
    .to_string()
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
  }

  for server in &manifest.mcp_servers {
    validate_manifest_identifier("mcp server", &server.id)?;
    if let Some(command) = server.command.as_ref() {
      if command.trim().is_empty() {
        anyhow::bail!(
          "plugin MCP server `{}` command must not be empty",
          server.id
        );
      }
    }
  }

  for connector in &manifest.app_connectors {
    validate_manifest_identifier("connector", &connector.id)?;
    if connector.service.trim().is_empty() {
      anyhow::bail!(
        "plugin connector `{}` must include a non-empty service",
        connector.id
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
  Ok(())
}

fn push_unique_capability(capabilities: &mut Vec<String>, kind: &str, identifier: &str) {
  let capability = format!("{kind}:{identifier}");
  if !capabilities.iter().any(|existing| existing == &capability) {
    capabilities.push(capability);
  }
}
