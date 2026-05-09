use super::io::{read_command_manifest, read_hook_manifest, read_manifest};
use super::validation::{manifest_capabilities, validate_manifest};
use std::path::PathBuf;

#[test]
fn bundled_plugin_manifests_match_runtime_schema() {
  let bundled_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../plugins/bundled");
  let manifests = [
    bundled_root.join("workspace-notes/pith-plugin.json"),
    bundled_root.join("shell-recorder/pith-plugin.json"),
    bundled_root.join("review-assistant/pith-plugin.json"),
    bundled_root.join("notion-connector/pith-plugin.json"),
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

  let hook_manifest = read_hook_manifest(
    &bundled_root.join("shell-recorder/hooks/shell.recorder.json"),
  )
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

  let notion_manifest = read_manifest(&bundled_root.join("notion-connector/pith-plugin.json"))
    .expect("parse notion connector manifest");
  let notion_capabilities = manifest_capabilities(&notion_manifest);
  assert!(notion_capabilities
    .iter()
    .any(|capability| capability == "connector:notion"));
  assert!(notion_capabilities
    .iter()
    .any(|capability| capability == "mcp_server:notion"));
}
