use pith_plugin_host::PluginHookEntry as HostPluginHookEntry;

pub(crate) fn plugin_hook_memory_tags(hook: &HostPluginHookEntry) -> Vec<String> {
  let mut tags = vec![
    "plugin".to_string(),
    "hook".to_string(),
    hook.plugin_id.clone(),
    hook.hook_id.clone(),
    hook.event.clone(),
  ];
  for tag in &hook.memory_note_tags {
    if !tags.iter().any(|existing| existing == tag) {
      tags.push(tag.clone());
    }
  }
  tags
}

#[cfg(test)]
mod tests {
  use super::*;

  fn hook() -> HostPluginHookEntry {
    HostPluginHookEntry {
      hook_id: "shell.recorder".to_string(),
      title: "Record Shell Completion".to_string(),
      description: "Record shell output".to_string(),
      event: "shell.completed".to_string(),
      message_template: "Command {{command}} exited with {{exitCode}}".to_string(),
      plugin_id: "shell-recorder".to_string(),
      plugin_display_name: "Shell Recorder".to_string(),
      permissions: vec!["shell.exec".to_string()],
      source_path: "/tmp/shell-recorder/pith-plugin.json".to_string(),
      memory_note_title: Some("Shell Completion".to_string()),
      memory_note_source: Some("plugin.shell-recorder".to_string()),
      memory_note_tags: vec!["shell".to_string(), "hook".to_string()],
    }
  }

  #[test]
  fn hook_memory_tags_keep_base_tags_and_deduplicate_manifest_tags() {
    let tags = plugin_hook_memory_tags(&hook());

    assert_eq!(
      tags,
      vec![
        "plugin".to_string(),
        "hook".to_string(),
        "shell-recorder".to_string(),
        "shell.recorder".to_string(),
        "shell.completed".to_string(),
        "shell".to_string(),
      ]
    );
  }
}
