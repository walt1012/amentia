use super::io::{read_command_manifest, read_hook_manifest, read_manifest};
use super::validation::{manifest_capabilities, validate_manifest};
use std::path::PathBuf;

#[test]
fn bundled_plugin_manifests_match_runtime_schema() {
  let bundled_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../plugins/bundled");
  let manifests = [
    bundled_root.join("workspace-notes/amentia-plugin.json"),
    bundled_root.join("shell-recorder/amentia-plugin.json"),
    bundled_root.join("review-assistant/amentia-plugin.json"),
    bundled_root.join("notion-connector/amentia-plugin.json"),
  ];

  for manifest_path in manifests {
    let manifest = read_manifest(&manifest_path).expect("parse bundled manifest");
    validate_manifest(&manifest).expect("validate bundled manifest");
    assert!(!manifest.display_name.trim().is_empty());
  }

  let workspace_command = read_command_manifest(
    &bundled_root.join("workspace-notes/commands/workspace.capture-note.json"),
  )
  .expect("parse workspace command manifest");
  assert_eq!(workspace_command.title, "Capture Workspace Note");
  assert_eq!(
    workspace_command
      .execution
      .as_ref()
      .map(|execution| execution.kind.as_str()),
    Some("builtin.workspaceReadmeNote")
  );
  assert_eq!(
    workspace_command
      .memory
      .as_ref()
      .map(|memory| memory.note_title.as_str()),
    Some("Workspace Capture")
  );

  let shell_command = read_command_manifest(
    &bundled_root.join("shell-recorder/commands/shell.summarize-session.json"),
  )
  .expect("parse shell command manifest");
  assert_eq!(shell_command.title, "Summarize Shell Session");
  assert_eq!(
    shell_command
      .execution
      .as_ref()
      .map(|execution| execution.kind.as_str()),
    Some("builtin.shellSessionSummary")
  );

  let review_command =
    read_command_manifest(&bundled_root.join("review-assistant/commands/review.inspect-diff.json"))
      .expect("parse review command manifest");
  assert_eq!(review_command.title, "Inspect Current Diff");
  assert_eq!(
    review_command
      .execution
      .as_ref()
      .map(|execution| execution.kind.as_str()),
    Some("builtin.reviewDiffSummary")
  );

  let hook_manifest =
    read_hook_manifest(&bundled_root.join("shell-recorder/hooks/shell.recorder.json"))
      .expect("parse bundled hook manifest");
  assert_eq!(hook_manifest.event, "shell.completed");
  assert!(!hook_manifest.message_template.trim().is_empty());
  assert_eq!(
    hook_manifest
      .memory
      .as_ref()
      .map(|memory| memory.note_title.as_str()),
    Some("Shell Completion")
  );

  let notion_manifest = read_manifest(&bundled_root.join("notion-connector/amentia-plugin.json"))
    .expect("parse notion connector manifest");
  let notion_capabilities = manifest_capabilities(&notion_manifest);
  assert!(notion_capabilities
    .iter()
    .any(|capability| capability == "connector:notion"));
  assert!(notion_capabilities
    .iter()
    .any(|capability| capability == "mcp_server:notion"));
  assert!(notion_capabilities
    .iter()
    .any(|capability| capability == "command:notion.prepare-page-draft"));
  assert!(notion_capabilities
    .iter()
    .any(|capability| capability == "command:notion.inspect-page-write"));
  assert!(notion_capabilities
    .iter()
    .any(|capability| capability == "command:notion.publish-page-draft"));
  assert!(notion_capabilities
    .iter()
    .any(|capability| capability == "connector_workflow:notion.create-page"));

  let notion_command = read_command_manifest(
    &bundled_root.join("notion-connector/commands/notion.prepare-page-draft.json"),
  )
  .expect("parse notion command manifest");
  assert_eq!(notion_command.title, "Prepare Notion Page Draft");
  assert_eq!(
    notion_command
      .execution
      .as_ref()
      .map(|execution| execution.kind.as_str()),
    Some("mcp.notion.preparePageDraft")
  );
  let notion_connectors = notion_command
    .execution
    .as_ref()
    .and_then(|execution| execution.connectors.as_ref())
    .expect("notion connector reference");
  assert_eq!(notion_connectors.len(), 1);
  assert_eq!(notion_connectors[0], "notion");
  assert_eq!(
    notion_command
      .execution
      .as_ref()
      .and_then(|execution| execution.workflow_id.as_deref()),
    Some("notion.create-page")
  );

  let notion_write_command = read_command_manifest(
    &bundled_root.join("notion-connector/commands/notion.inspect-page-write.json"),
  )
  .expect("parse notion write inspection command manifest");
  assert_eq!(notion_write_command.title, "Inspect Notion Page Write");
  assert_eq!(
    notion_write_command
      .execution
      .as_ref()
      .map(|execution| execution.kind.as_str()),
    Some("mcp.notion.inspectPageWrite")
  );
  assert_eq!(
    notion_write_command
      .execution
      .as_ref()
      .and_then(|execution| execution.workflow_id.as_deref()),
    Some("notion.create-page")
  );

  let notion_publish_command = read_command_manifest(
    &bundled_root.join("notion-connector/commands/notion.publish-page-draft.json"),
  )
  .expect("parse notion publish command manifest");
  assert_eq!(notion_publish_command.title, "Publish Notion Page Draft");
  assert_eq!(
    notion_publish_command
      .execution
      .as_ref()
      .map(|execution| execution.kind.as_str()),
    Some("mcp.notion.publishPageDraft")
  );
  assert_eq!(
    notion_publish_command
      .execution
      .as_ref()
      .and_then(|execution| execution.workflow_id.as_deref()),
    Some("notion.create-page")
  );
}
