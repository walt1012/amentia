use std::collections::HashMap;

pub(super) fn readiness_repair_hint(run_status: &str, run_blocker: &str) -> String {
  match run_status {
    "invalidCommandManifest" => {
      "Fix the command manifest JSON and schema fields, then refresh plugins.".to_string()
    }
    "missingExecution" => "Add an execution contract before running this command.".to_string(),
    "unsupportedExecution" => "Use a supported driver: builtin, stdio, or MCP stdio.".to_string(),
    "missingConnector" => {
      "Declare the connector in the plugin manifest or remove it from the command contract."
        .to_string()
    }
    "missingPermission" => {
      "Add the required permission to the plugin manifest or narrow the command capability."
        .to_string()
    }
    "runnerSetup" => runner_setup_repair_hint(run_blocker),
    "needsConnectorAuth" => "Authorize the connector before running this command.".to_string(),
    _ => "Inspect the plugin manifest and command contract, then refresh plugins.".to_string(),
  }
}

pub(super) fn runner_failure_recovery_hint(
  failure_kind: &str,
  attributes: &HashMap<String, String>,
) -> String {
  match failure_kind {
    "cancelled" => "Run the command again when the current task is ready.".to_string(),
    "timeout" => {
      "Ensure the runner reads stdin, avoids unbounded work, and exits within the plugin timeout."
        .to_string()
    }
    "unsupportedExecution" => {
      "Update the plugin command manifest to declare a supported execution contract.".to_string()
    }
    "mcpProtocol" => mcp_protocol_recovery_hint(attributes),
    "outputContract" => output_contract_recovery_hint(attributes),
    "processExit" => {
      "Fix the runner error shown in stderr/stdout, then return exit code 0 with valid output."
        .to_string()
    }
    "runnerSetup" => runner_setup_recovery_hint(attributes),
    _ => "Check the plugin manifest, entrypoint path, sandbox, and local files.".to_string(),
  }
}

fn runner_setup_recovery_hint(attributes: &HashMap<String, String>) -> String {
  if let Some(check) = attributes.get("pluginRunnerEntrypointCheck") {
    match check.as_str() {
      "metadataError" => {
        return "Check that the runner entrypoint exists and is readable, then refresh plugins."
          .to_string();
      }
      "notFile" => {
        return "Point the runner entrypoint at an executable file inside the plugin bundle."
          .to_string();
      }
      "notExecutable" => {
        return "Mark the runner entrypoint executable, then refresh plugins.".to_string();
      }
      _ => {}
    }
  }

  let detail = attributes
    .get("pluginRunnerSetupDetail")
    .map(String::as_str)
    .unwrap_or_default();
  runner_setup_repair_hint(detail)
}

fn output_contract_recovery_hint(attributes: &HashMap<String, String>) -> String {
  match attributes.get("pluginRunnerOutputStatus").map(String::as_str) {
    Some("malformedEnvelope") => {
      "Return valid JSON with `content`, `message`, `items`, or `memoryNotes`, or return plain text."
        .to_string()
    }
    Some("emptyEnvelope") => {
      "Populate `content`, `message`, `items`, or `memoryNotes`, or return plain text.".to_string()
    }
    Some("invalidEnvelope") => {
      "Fix invalid timeline items or memory notes; each item needs kind, title, and content, and each memory note needs title and body."
        .to_string()
    }
    _ => "Return plain text, a valid JSON output envelope, valid timeline items, or memory notes."
      .to_string(),
  }
}

fn mcp_protocol_recovery_hint(attributes: &HashMap<String, String>) -> String {
  match attributes.get("mcpProtocolStatus").map(String::as_str) {
    Some("missingToolResponse") => {
      if attribute_number_is_positive(attributes, "mcpInvalidJsonLineCount") {
        return "Keep MCP stdout reserved for JSON-RPC responses, move logs to stderr, and return the tools/call response with the expected request id."
          .to_string();
      }
      if attributes
        .get("mcpInitializeResponseSeen")
        .is_some_and(|seen| seen == "false")
      {
        return "Handle the MCP initialize request before returning the tools/call response."
          .to_string();
      }
      "Make the MCP server return a JSON-RPC tools/call response with the expected request id."
        .to_string()
    }
    Some("toolError") | Some("toolResultError") => {
      "Fix the MCP tool error and return a successful tool result.".to_string()
    }
    Some("missingResult") => "Return an MCP tool response with a result object.".to_string(),
    Some("unsupportedContent") => {
      "Return MCP text content or structuredContent that Pith can convert into text, timeline items, or memory notes."
        .to_string()
    }
    Some("emptyContent") => {
      "Return non-empty MCP text content or structuredContent that Pith can convert into a useful plugin result."
        .to_string()
    }
    _ => "Check the MCP server command and stdout JSON-RPC framing.".to_string(),
  }
}

fn attribute_number_is_positive(attributes: &HashMap<String, String>, key: &str) -> bool {
  attributes
    .get(key)
    .and_then(|value| value.parse::<usize>().ok())
    .is_some_and(|value| value > 0)
}

fn runner_setup_repair_hint(detail: &str) -> String {
  let detail = detail.to_ascii_lowercase();
  if detail.contains("not executable") {
    return "Mark the runner entrypoint executable, then refresh plugins.".to_string();
  }
  if detail.contains("is not a file") {
    return "Point the runner entrypoint at an executable file inside the plugin bundle."
      .to_string();
  }
  if detail.contains("metadata could not be read") {
    return "Check that the runner entrypoint exists and is readable, then refresh plugins."
      .to_string();
  }
  if detail.contains("entrypoint could not be resolved") {
    return "Add the runner file to the plugin bundle or update the entrypoint path.".to_string();
  }
  if detail.contains("must stay inside") || detail.contains("resolved outside") {
    return "Use a relative entrypoint path that stays inside the plugin bundle.".to_string();
  }
  if detail.contains("mcp server") && detail.contains("not declared") {
    return "Declare the referenced MCP server in the plugin manifest or update the command entrypoint."
      .to_string();
  }
  if detail.contains("mcp server command") {
    return "Add an MCP server command to the plugin manifest or switch the command driver."
      .to_string();
  }
  if detail.contains("plugin root") || detail.contains("valid plugin root") {
    return "Reinstall the plugin from a valid bundle so source paths resolve.".to_string();
  }

  "Check the runner path, executable bit, local bundle files, and sandbox diagnostics.".to_string()
}

#[cfg(test)]
mod tests {
  use std::collections::HashMap;

  use super::{readiness_repair_hint, runner_failure_recovery_hint};

  #[test]
  fn readiness_hint_explains_missing_runner_file() {
    let hint = readiness_repair_hint(
      "runnerSetup",
      "Plugin runner entrypoint could not be resolved: No such file or directory",
    );

    assert!(hint.contains("Add the runner file"));
  }

  #[test]
  fn failure_hint_explains_non_executable_entrypoint() {
    let attributes = HashMap::from([(
      "pluginRunnerEntrypointCheck".to_string(),
      "notExecutable".to_string(),
    )]);

    let hint = runner_failure_recovery_hint("runnerSetup", &attributes);

    assert!(hint.contains("Mark the runner entrypoint executable"));
  }

  #[test]
  fn output_hint_explains_empty_envelope() {
    let attributes = HashMap::from([(
      "pluginRunnerOutputStatus".to_string(),
      "emptyEnvelope".to_string(),
    )]);

    let hint = runner_failure_recovery_hint("outputContract", &attributes);

    assert!(hint.contains("Populate"));
  }

  #[test]
  fn output_hint_explains_invalid_envelope_items() {
    let attributes = HashMap::from([(
      "pluginRunnerOutputStatus".to_string(),
      "invalidEnvelope".to_string(),
    )]);

    let hint = runner_failure_recovery_hint("outputContract", &attributes);

    assert!(hint.contains("kind, title, and content"));
    assert!(hint.contains("title and body"));
  }

  #[test]
  fn mcp_hint_explains_stdout_json_rpc_framing() {
    let attributes = HashMap::from([
      (
        "mcpProtocolStatus".to_string(),
        "missingToolResponse".to_string(),
      ),
      ("mcpInvalidJsonLineCount".to_string(), "2".to_string()),
    ]);

    let hint = runner_failure_recovery_hint("mcpProtocol", &attributes);

    assert!(hint.contains("stdout reserved for JSON-RPC"));
    assert!(hint.contains("logs to stderr"));
  }

  #[test]
  fn mcp_hint_explains_missing_initialize_response() {
    let attributes = HashMap::from([
      (
        "mcpProtocolStatus".to_string(),
        "missingToolResponse".to_string(),
      ),
      ("mcpInitializeResponseSeen".to_string(), "false".to_string()),
    ]);

    let hint = runner_failure_recovery_hint("mcpProtocol", &attributes);

    assert!(hint.contains("initialize request"));
  }

  #[test]
  fn mcp_hint_explains_unsupported_content() {
    let attributes = HashMap::from([(
      "mcpProtocolStatus".to_string(),
      "unsupportedContent".to_string(),
    )]);

    let hint = runner_failure_recovery_hint("mcpProtocol", &attributes);

    assert!(hint.contains("text content"));
    assert!(hint.contains("structuredContent"));
  }

  #[test]
  fn mcp_hint_explains_empty_content() {
    let attributes = HashMap::from([(
      "mcpProtocolStatus".to_string(),
      "emptyContent".to_string(),
    )]);

    let hint = runner_failure_recovery_hint("mcpProtocol", &attributes);

    assert!(hint.contains("non-empty"));
    assert!(hint.contains("useful plugin result"));
  }
}
