use pith_plugin_host::PluginCommandEntry as HostPluginCommandEntry;

pub(super) fn plugin_command_memory_tags(command: &HostPluginCommandEntry) -> Vec<String> {
  let mut tags = vec![
    "plugin".to_string(),
    "command".to_string(),
    command.plugin_id.clone(),
    command.command_id.clone(),
  ];
  for tag in &command.memory_note_tags {
    if !tags.iter().any(|existing| existing == tag) {
      tags.push(tag.clone());
    }
  }
  tags
}

#[cfg(test)]
mod tests {
  use super::*;
  use pith_plugin_host::{
    PluginCommandEnvelopeEntry as HostPluginCommandEnvelopeEntry,
    PluginCommandExecutionEntry as HostPluginCommandExecutionEntry,
  };

  fn command() -> HostPluginCommandEntry {
    HostPluginCommandEntry {
      command_id: "capture.readme".to_string(),
      title: "Capture README".to_string(),
      description: "Capture README context".to_string(),
      prompt: "Capture README".to_string(),
      plugin_id: "workspace-notes".to_string(),
      plugin_display_name: "Workspace Notes".to_string(),
      permissions: vec!["file.read".to_string()],
      source_path: "/tmp/workspace-notes/pith-plugin.json".to_string(),
      execution: Some(HostPluginCommandExecutionEntry {
        kind: "builtin.workspaceReadmeNote".to_string(),
        driver: "builtin".to_string(),
        entrypoint: None,
        connector_ids: None,
        workflow_id: None,
        input: empty_envelope("pith.plugin.command.input"),
        output: empty_envelope("pith.plugin.command.output"),
      }),
      execution_kind: Some("builtin.workspaceReadmeNote".to_string()),
      manifest_error: None,
      memory_note_title: Some("README Note".to_string()),
      memory_note_source: Some("plugin.workspace-notes".to_string()),
      memory_note_tags: vec!["readme".to_string(), "command".to_string()],
    }
  }

  fn empty_envelope(envelope: &str) -> HostPluginCommandEnvelopeEntry {
    HostPluginCommandEnvelopeEntry {
      envelope: envelope.to_string(),
      fields: vec![],
    }
  }

  #[test]
  fn command_memory_tags_keep_base_tags_and_deduplicate_manifest_tags() {
    let tags = plugin_command_memory_tags(&command());

    assert_eq!(
      tags,
      vec![
        "plugin".to_string(),
        "command".to_string(),
        "workspace-notes".to_string(),
        "capture.readme".to_string(),
        "readme".to_string(),
      ]
    );
  }
}
