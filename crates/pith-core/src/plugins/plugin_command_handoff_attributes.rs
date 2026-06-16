use std::collections::HashMap;

const OBSERVATION_HANDOFF_KEYS: &[&str] = &[
  "connectorId",
  "connectorIds",
  "connectorServices",
  "targetService",
  "targetTool",
  "draftMode",
  "remoteWrite",
  "remoteWriteStage",
  "remoteWriteStatus",
  "remoteWriteRequiresApproval",
  "remoteProofKind",
  "remoteProofStatus",
  "remoteProofId",
  "remoteProofUrl",
  "remoteProofTitle",
  "remoteProofActionTitle",
  "remoteProofCopyTitle",
  "notionPageId",
  "notionPageUrl",
  "notionParentPageId",
  "notionBlockCount",
  "bodyTruncated",
  "sourceArtifact",
  "sourceArtifactPreviewProvided",
  "publishRetryable",
  "publishFailureReason",
  "retryCommandId",
  "retryInput",
  "connectorWorkflowId",
  "connectorWorkflowName",
  "connectorWorkflowService",
  "connectorWorkflowAction",
  "connectorWorkflowStage",
  "connectorWorkflowStatus",
  "connectorWorkflowTarget",
  "connectorWorkflowProof",
  "connectorWorkflowRecovery",
];
const RUNNER_CONNECTOR_HANDOFF_KEYS: &[(&str, &str)] = &[
  ("pluginRunnerConnectorId", "connectorId"),
  ("pluginRunnerConnectorIds", "connectorIds"),
  ("pluginRunnerConnectorServices", "connectorServices"),
];

pub(super) fn copy_observation_handoff_attributes(
  attributes: &mut HashMap<String, String>,
  observation_attributes: Option<&HashMap<String, String>>,
) {
  let Some(observation_attributes) = observation_attributes else {
    return;
  };
  for key in OBSERVATION_HANDOFF_KEYS.iter().copied() {
    if let Some(value) = observation_attributes.get(key) {
      attributes.insert(key.to_string(), value.clone());
    }
  }
  for (source_key, target_key) in RUNNER_CONNECTOR_HANDOFF_KEYS.iter().copied() {
    if let Some(value) = observation_attributes.get(source_key) {
      attributes.insert(target_key.to_string(), value.clone());
    }
  }
}

#[cfg(test)]
mod tests {
  use std::collections::HashMap;

  use super::copy_observation_handoff_attributes;

  #[test]
  fn copies_reviewable_connector_evidence_only() {
    let mut attributes = HashMap::new();
    let observation_attributes = HashMap::from([
      ("remoteProofUrl".to_string(), "https://example.com/proof".to_string()),
      ("connectorWorkflowStatus".to_string(), "prepared".to_string()),
      ("pluginRunnerOutputStatus".to_string(), "envelope".to_string()),
      ("internalTrace".to_string(), "hidden".to_string()),
    ]);

    copy_observation_handoff_attributes(&mut attributes, Some(&observation_attributes));

    assert_eq!(
      attributes.get("remoteProofUrl").map(String::as_str),
      Some("https://example.com/proof")
    );
    assert_eq!(
      attributes
        .get("connectorWorkflowStatus")
        .map(String::as_str),
      Some("prepared")
    );
    assert!(!attributes.contains_key("pluginRunnerOutputStatus"));
    assert!(!attributes.contains_key("internalTrace"));
  }

  #[test]
  fn maps_runner_connector_metadata_for_handoff() {
    let mut attributes = HashMap::new();
    let observation_attributes = HashMap::from([(
      "pluginRunnerConnectorServices".to_string(),
      "notion,calendar".to_string(),
    )]);

    copy_observation_handoff_attributes(&mut attributes, Some(&observation_attributes));

    assert_eq!(
      attributes.get("connectorServices").map(String::as_str),
      Some("notion,calendar")
    );
  }
}
