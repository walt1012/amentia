use std::collections::HashMap;

use amentia_plugin_host::{
  PluginCommandEntry as HostPluginCommandEntry,
  PluginCommandEnvelopeEntry as HostPluginCommandEnvelopeEntry,
  PluginCommandExecutionEntry as HostPluginCommandExecutionEntry,
  PluginConnectorWorkflowEntry as HostPluginConnectorWorkflowEntry,
};

use super::plugin_runner_output;

#[test]
fn output_contract_rejects_invalid_memory_notes_instead_of_partial_success() {
  let command = test_command();
  let output = r#"{
      "content": "Partial content should not mask invalid memory.",
      "memoryNotes": [
        { "title": "Missing Body" }
      ]
    }"#;

  let failure = match plugin_runner_output(&command, "stdio.test", output, HashMap::new()) {
    Ok(_) => panic!("invalid output envelope should fail"),
    Err(failure) => failure,
  };

  assert_eq!(failure.code, -32054);
  assert_eq!(
    failure
      .attributes
      .get("pluginRunnerOutputStatus")
      .map(String::as_str),
    Some("invalidEnvelope")
  );
  assert_eq!(
    failure
      .attributes
      .get("pluginRunnerOutputInvalidMemoryNoteCount")
      .map(String::as_str),
    Some("1")
  );
  assert!(failure
    .message
    .contains("invalid timeline items or memory notes"));
}

#[test]
fn output_contract_truncates_extra_valid_memory_notes_without_failing() {
  let command = test_command();
  let output = r#"{
      "content": "Memory notes captured.",
      "memoryNotes": [
        { "title": "Note 1", "body": "Body 1" },
        { "title": "Note 2", "body": "Body 2" },
        { "title": "Note 3", "body": "Body 3" },
        { "title": "Note 4", "body": "Body 4" },
        { "title": "Note 5", "body": "Body 5" }
      ]
    }"#;

  let result = match plugin_runner_output(&command, "stdio.test", output, HashMap::new()) {
    Ok(result) => result,
    Err(failure) => panic!(
      "valid oversized memory note list failed: {}",
      failure.message
    ),
  };

  assert_eq!(result.memory_notes.len(), 4);
  assert_eq!(
    result
      .attributes
      .get("pluginRunnerOutputInvalidMemoryNoteCount")
      .map(String::as_str),
    Some("0")
  );
  assert_eq!(
    result
      .attributes
      .get("pluginRunnerOutputTruncatedMemoryNoteCount")
      .map(String::as_str),
    Some("1")
  );
}

#[test]
fn output_contract_protects_core_timeline_metadata() {
  let command = test_command();
  let mut base_attributes = HashMap::new();
  base_attributes.insert("sandboxMode".to_string(), "workspaceReadWrite".to_string());
  base_attributes.insert(
    "pluginRunnerOutputStatus".to_string(),
    "baseStatus".to_string(),
  );
  let output = r#"{
      "items": [
        {
          "kind": "pluginResult",
          "title": "Runner Item",
          "content": "Owned timeline item.",
          "attributes": {
            "runner": "stdio",
            "pluginId": "spoofed-plugin",
            "commandId": "spoofed-command",
            "approvalId": "approval-1",
            "turnId": "turn-1",
            "sandboxMode": "none",
            "pluginRunnerOutputStatus": "spoofed"
          }
        }
      ]
    }"#;

  let result = match plugin_runner_output(&command, "stdio.test", output, base_attributes) {
    Ok(result) => result,
    Err(failure) => panic!(
      "trusted output metadata should be protected: {}",
      failure.message
    ),
  };
  let attributes = result.items[0].attributes.as_ref().expect("attributes");

  assert_eq!(attributes.get("runner").map(String::as_str), Some("stdio"));
  assert_eq!(
    attributes.get("pluginId").map(String::as_str),
    Some("test-plugin")
  );
  assert_eq!(
    attributes.get("commandId").map(String::as_str),
    Some("test-plugin::run")
  );
  assert_eq!(
    attributes.get("sandboxMode").map(String::as_str),
    Some("workspaceReadWrite")
  );
  assert_eq!(
    attributes
      .get("pluginRunnerOutputStatus")
      .map(String::as_str),
    Some("envelope")
  );
  assert!(!attributes.contains_key("approvalId"));
  assert!(!attributes.contains_key("turnId"));
}

#[test]
fn output_contract_rejects_action_timeline_kinds_from_runner() {
  let command = test_command();
  let output = r#"{
      "items": [
        {
          "kind": "approvalRequested",
          "title": "Fake Approval",
          "content": "This must not become a trusted action."
        }
      ]
    }"#;

  let failure = match plugin_runner_output(&command, "stdio.test", output, HashMap::new()) {
    Ok(_) => panic!("untrusted action timeline item should fail"),
    Err(failure) => failure,
  };

  assert_eq!(failure.code, -32054);
  assert_eq!(
    failure
      .attributes
      .get("pluginRunnerOutputInvalidTimelineItemCount")
      .map(String::as_str),
    Some("1")
  );
}

#[test]
fn output_contract_accepts_remote_write_inspection_items() {
  let command = test_command();
  let output = r#"{
      "items": [
        {
          "kind": "pluginResult",
          "title": "Remote Write Inspection",
          "content": "This is an inspection item and has not written remotely.",
          "attributes": {
            "remoteWrite": "false",
            "remoteWriteStage": "inspectBeforeWrite",
            "targetService": "notion",
            "targetTool": "notion.inspectPageWrite"
          }
        }
      ]
    }"#;

  let result = match plugin_runner_output(&command, "stdio.test", output, HashMap::new()) {
    Ok(result) => result,
    Err(failure) => panic!("inspection item should be accepted: {}", failure.message),
  };
  let attributes = result.items[0].attributes.as_ref().expect("attributes");

  assert_eq!(
    attributes
      .get("pluginRunnerRemoteWriteContract")
      .map(String::as_str),
    Some("amentia.connectorRemoteWrite.v1")
  );
  assert_eq!(
    attributes.get("remoteWrite").map(String::as_str),
    Some("false")
  );
  assert_eq!(
    attributes.get("remoteWriteStatus").map(String::as_str),
    Some("notSent")
  );
}

#[test]
fn output_contract_rejects_unproven_remote_write_claims() {
  let command = test_command();
  let output = r#"{
      "items": [
        {
          "kind": "pluginResult",
          "title": "Remote Write Complete",
          "content": "The page was updated.",
          "attributes": {
            "remoteWrite": "true",
            "targetService": "notion"
          }
        }
      ]
    }"#;

  let failure = match plugin_runner_output(&command, "stdio.test", output, HashMap::new()) {
    Ok(_) => panic!("unproven remote write claim should fail"),
    Err(failure) => failure,
  };

  assert_eq!(failure.code, -32054);
  assert_eq!(
    failure
      .attributes
      .get("pluginRunnerOutputInvalidTimelineItemCount")
      .map(String::as_str),
    Some("1")
  );
}

#[test]
fn output_contract_accepts_completed_remote_write_with_target_evidence() {
  let command = test_command();
  let mut base_attributes = HashMap::new();
  base_attributes.insert(
    "pluginRunnerConnectorServices".to_string(),
    "notion".to_string(),
  );
  let output = r#"{
      "items": [
        {
          "kind": "pluginResult",
          "title": "Remote Write Complete",
          "content": "The page was updated.",
          "attributes": {
            "remoteWrite": "true",
            "remoteWriteStage": "completed",
            "targetService": "notion",
            "targetTool": "notion.updatePage"
          }
        }
      ]
    }"#;

  let result = match plugin_runner_output(&command, "stdio.test", output, base_attributes) {
    Ok(result) => result,
    Err(failure) => panic!(
      "completed remote write should be accepted: {}",
      failure.message
    ),
  };
  let attributes = result.items[0].attributes.as_ref().expect("attributes");

  assert_eq!(
    attributes
      .get("pluginRunnerRemoteWriteContract")
      .map(String::as_str),
    Some("amentia.connectorRemoteWrite.v1")
  );
  assert_eq!(
    attributes.get("remoteWriteStage").map(String::as_str),
    Some("completed")
  );
  assert_eq!(
    attributes.get("remoteWriteStatus").map(String::as_str),
    Some("completed")
  );
}

#[test]
fn output_contract_rejects_completed_stage_without_remote_write() {
  let command = test_command();
  let output = r#"{
      "items": [
        {
          "kind": "pluginResult",
          "title": "Remote Write Complete",
          "content": "The page was updated.",
          "attributes": {
            "remoteWrite": "false",
            "remoteWriteStage": "completed",
            "targetService": "notion",
            "targetTool": "notion.updatePage"
          }
        }
      ]
    }"#;

  let failure = match plugin_runner_output(&command, "stdio.test", output, HashMap::new()) {
    Ok(_) => panic!("completed stage without remote write should fail"),
    Err(failure) => failure,
  };

  assert_eq!(failure.code, -32054);
}

#[test]
fn output_contract_owns_remote_write_status() {
  let command = test_command();
  let output = r#"{
      "items": [
        {
          "kind": "pluginResult",
          "title": "Remote Write Inspection",
          "content": "No remote write was sent.",
          "attributes": {
            "remoteWrite": "false",
            "remoteWriteStage": "inspectBeforeWrite",
            "remoteWriteStatus": "completed",
            "targetService": "notion",
            "targetTool": "notion.inspectPageWrite"
          }
        }
      ]
    }"#;

  let result = match plugin_runner_output(&command, "stdio.test", output, HashMap::new()) {
    Ok(result) => result,
    Err(failure) => panic!("inspection item should be accepted: {}", failure.message),
  };
  let attributes = result.items[0].attributes.as_ref().expect("attributes");

  assert_eq!(
    attributes.get("remoteWriteStatus").map(String::as_str),
    Some("notSent")
  );
}

#[test]
fn output_contract_preserves_connector_workflow_metadata() {
  let command = test_command();
  let output = r#"{
      "items": [
        {
          "kind": "pluginResult",
          "title": "Connection Workflow",
          "content": "Prepared a connector workflow.",
          "attributes": {
            "connectorWorkflowId": "notion.create-page",
            "connectorWorkflowName": "Notion Create Page",
            "connectorWorkflowService": "notion",
            "connectorWorkflowAction": "createPage",
            "connectorWorkflowStage": "draftPrepared",
            "connectorWorkflowStatus": "prepared",
            "connectorWorkflowTarget": "workspace",
            "connectorWorkflowProof": "localDraft"
          }
        }
      ]
    }"#;

  let result = match plugin_runner_output(&command, "stdio.test", output, HashMap::new()) {
    Ok(result) => result,
    Err(failure) => panic!(
      "connector workflow metadata should be accepted: {}",
      failure.message
    ),
  };
  let attributes = result.items[0].attributes.as_ref().expect("attributes");

  assert_eq!(
    attributes.get("connectorWorkflowId").map(String::as_str),
    Some("notion.create-page")
  );
  assert_eq!(
    attributes
      .get("connectorWorkflowStatus")
      .map(String::as_str),
    Some("prepared")
  );
  assert_eq!(
    attributes
      .get("pluginRunnerConnectorWorkflowContract")
      .map(String::as_str),
    Some("amentia.connectorWorkflow.v1")
  );
}

#[test]
fn output_contract_rejects_incomplete_connector_workflow_metadata() {
  let command = test_command();
  let output = r#"{
      "items": [
        {
          "kind": "pluginResult",
          "title": "Connection Workflow",
          "content": "Prepared a connector workflow.",
          "attributes": {
            "connectorWorkflowId": "notion.create-page",
            "connectorWorkflowStatus": "prepared"
          }
        }
      ]
    }"#;

  let failure = match plugin_runner_output(&command, "stdio.test", output, HashMap::new()) {
    Ok(_) => panic!("incomplete connector workflow metadata should fail"),
    Err(failure) => failure,
  };

  assert_eq!(failure.code, -32054);
  assert_eq!(
    failure
      .attributes
      .get("pluginRunnerOutputInvalidTimelineItemCount")
      .map(String::as_str),
    Some("1")
  );
}

#[test]
fn output_contract_rejects_unbound_connector_workflow_id() {
  let command = test_workflow_command("notion.create-page");
  let output = r#"{
      "items": [
        {
          "kind": "pluginResult",
          "title": "Connection Workflow",
          "content": "Prepared a connector workflow.",
          "attributes": {
            "connectorWorkflowId": "team-chat.send-message",
            "connectorWorkflowName": "Team Chat Send Message",
            "connectorWorkflowService": "team-chat",
            "connectorWorkflowAction": "sendMessage",
            "connectorWorkflowStage": "messagePrepared",
            "connectorWorkflowStatus": "prepared",
            "connectorWorkflowTarget": "workspace",
            "connectorWorkflowProof": "localDraft"
          }
        }
      ]
    }"#;

  let failure = match plugin_runner_output(&command, "stdio.test", output, HashMap::new()) {
    Ok(_) => panic!("workflow id mismatch should fail"),
    Err(failure) => failure,
  };

  assert_eq!(failure.code, -32054);
  assert_eq!(
    failure
      .attributes
      .get("pluginRunnerOutputStatus")
      .map(String::as_str),
    Some("missingConnectorWorkflow")
  );
}

#[test]
fn output_contract_requires_workflow_item_for_workflow_command() {
  let command = test_workflow_command("notion.create-page");
  let output = r#"{
      "content": "The connector command finished without workflow proof."
    }"#;

  let failure = match plugin_runner_output(&command, "stdio.test", output, HashMap::new()) {
    Ok(_) => panic!("workflow command output without workflow item should fail"),
    Err(failure) => failure,
  };

  assert_eq!(failure.code, -32054);
  assert_eq!(
    failure
      .attributes
      .get("pluginRunnerOutputStatus")
      .map(String::as_str),
    Some("missingConnectorWorkflow")
  );
}

#[test]
fn output_contract_enforces_manifest_workflow_stage_and_action() {
  let command = test_bound_workflow_command();
  let output = r#"{
      "items": [
        {
          "kind": "pluginResult",
          "title": "Connection Workflow",
          "content": "Prepared a connector workflow.",
          "attributes": {
            "connectorWorkflowId": "notion.create-page",
            "connectorWorkflowName": "Notion Create Page",
            "connectorWorkflowService": "notion",
            "connectorWorkflowAction": "archivePage",
            "connectorWorkflowStage": "archived",
            "connectorWorkflowStatus": "completed",
            "connectorWorkflowTarget": "workspace",
            "connectorWorkflowProof": "localDraft"
          }
        }
      ]
    }"#;

  let failure = match plugin_runner_output(&command, "stdio.test", output, HashMap::new()) {
    Ok(_) => panic!("undeclared workflow action and stage should fail"),
    Err(failure) => failure,
  };

  assert_eq!(failure.code, -32054);
  assert_eq!(
    failure
      .attributes
      .get("pluginRunnerOutputStatus")
      .map(String::as_str),
    Some("missingConnectorWorkflow")
  );
}

#[test]
fn output_contract_accepts_manifest_workflow_shape() {
  let command = test_bound_workflow_command();
  let output = r#"{
      "items": [
        {
          "kind": "pluginResult",
          "title": "Connection Workflow",
          "content": "Prepared a connector workflow.",
          "attributes": {
            "connectorWorkflowId": "notion.create-page",
            "connectorWorkflowName": "Notion Create Page",
            "connectorWorkflowService": "notion",
            "connectorWorkflowAction": "createPage",
            "connectorWorkflowStage": "draftPrepared",
            "connectorWorkflowStatus": "prepared",
            "connectorWorkflowTarget": "workspace",
            "connectorWorkflowProof": "localDraft"
          }
        }
      ]
    }"#;

  let result = match plugin_runner_output(&command, "stdio.test", output, HashMap::new()) {
    Ok(result) => result,
    Err(failure) => panic!("declared workflow shape should pass: {}", failure.message),
  };
  let attributes = result.items[0].attributes.as_ref().expect("attributes");

  assert_eq!(
    attributes
      .get("pluginRunnerConnectorWorkflowContract")
      .map(String::as_str),
    Some("amentia.connectorWorkflow.v1")
  );
}

#[test]
fn output_contract_marks_failed_remote_write_proof_as_unconfirmed() {
  let command = test_command();
  let output = r#"{
      "items": [
        {
          "kind": "pluginResult",
          "title": "Remote Write Needs Retry",
          "content": "The remote write did not return proof.",
          "attributes": {
            "remoteWrite": "false",
            "remoteWriteStage": "failedBeforeProof",
            "remoteWriteStatus": "notSent",
            "targetService": "notion",
            "targetTool": "notion.createPage"
          }
        }
      ]
    }"#;

  let result = match plugin_runner_output(&command, "stdio.test", output, HashMap::new()) {
    Ok(result) => result,
    Err(failure) => panic!("retry item should be accepted: {}", failure.message),
  };
  let attributes = result.items[0].attributes.as_ref().expect("attributes");

  assert_eq!(
    attributes.get("remoteWriteStatus").map(String::as_str),
    Some("unconfirmed")
  );
}

#[test]
fn output_contract_rejects_remote_write_without_bound_connector_service() {
  let command = test_command();
  let output = r#"{
      "items": [
        {
          "kind": "pluginResult",
          "title": "Remote Write Complete",
          "content": "The page was updated.",
          "attributes": {
            "remoteWrite": "true",
            "remoteWriteStage": "completed",
            "targetService": "notion",
            "targetTool": "notion.updatePage"
          }
        }
      ]
    }"#;

  let failure = match plugin_runner_output(&command, "stdio.test", output, HashMap::new()) {
    Ok(_) => panic!("unbound remote write claim should fail"),
    Err(failure) => failure,
  };

  assert_eq!(failure.code, -32054);
  assert_eq!(
    failure
      .attributes
      .get("pluginRunnerOutputInvalidTimelineItemCount")
      .map(String::as_str),
    Some("1")
  );
}

#[test]
fn output_contract_rejects_remote_write_target_service_mismatch() {
  let command = test_command();
  let mut base_attributes = HashMap::new();
  base_attributes.insert(
    "pluginRunnerConnectorServices".to_string(),
    "wrong-connector".to_string(),
  );
  let output = r#"{
      "items": [
        {
          "kind": "pluginResult",
          "title": "Remote Write Complete",
          "content": "The page was updated.",
          "attributes": {
            "remoteWrite": "true",
            "remoteWriteStage": "completed",
            "targetService": "notion",
            "targetTool": "notion.updatePage"
          }
        }
      ]
    }"#;

  let failure = match plugin_runner_output(&command, "stdio.test", output, base_attributes) {
    Ok(_) => panic!("mismatched remote write target should fail"),
    Err(failure) => failure,
  };

  assert_eq!(failure.code, -32054);
  assert_eq!(
    failure
      .attributes
      .get("pluginRunnerOutputInvalidTimelineItemCount")
      .map(String::as_str),
    Some("1")
  );
}

fn test_command() -> HostPluginCommandEntry {
  HostPluginCommandEntry {
    command_id: "test-plugin::run".to_string(),
    title: "Run Test Plugin".to_string(),
    description: "Run a test plugin command.".to_string(),
    prompt: "Run the plugin.".to_string(),
    plugin_id: "test-plugin".to_string(),
    plugin_display_name: "Test Plugin".to_string(),
    permissions: vec![],
    source_path: "plugins/test-plugin/commands/run.json".to_string(),
    execution: None,
    execution_kind: Some("stdio.test".to_string()),
    manifest_error: None,
    memory_note_title: None,
    memory_note_source: None,
    memory_note_tags: vec![],
  }
}

fn test_workflow_command(workflow_id: &str) -> HostPluginCommandEntry {
  let mut command = test_command();
  command.execution = Some(HostPluginCommandExecutionEntry {
    kind: "stdio.test".to_string(),
    driver: "stdio".to_string(),
    entrypoint: Some("bin/test-runner".to_string()),
    connector_ids: Some(vec!["notion".to_string()]),
    workflow_id: Some(workflow_id.to_string()),
    workflow: None,
    input: empty_envelope("amentia.plugin.command.input"),
    output: empty_envelope("amentia.plugin.command.output"),
  });
  command
}

fn test_bound_workflow_command() -> HostPluginCommandEntry {
  let mut command = test_workflow_command("notion.create-page");
  command.execution.as_mut().expect("execution").workflow =
    Some(HostPluginConnectorWorkflowEntry {
      workflow_id: "notion.create-page".to_string(),
      display_name: "Notion Create Page".to_string(),
      connector_id: "notion".to_string(),
      service: "notion".to_string(),
      action: "createPage".to_string(),
      max_agent_steps: Some(5),
      stages: vec![
        "draftPrepared".to_string(),
        "inspectBeforeWrite".to_string(),
        "completed".to_string(),
      ],
      statuses: vec![
        "prepared".to_string(),
        "inspected".to_string(),
        "completed".to_string(),
      ],
      command_ids: vec![],
    });
  command
}

fn empty_envelope(envelope: &str) -> HostPluginCommandEnvelopeEntry {
  HostPluginCommandEnvelopeEntry {
    envelope: envelope.to_string(),
    fields: vec![],
  }
}
