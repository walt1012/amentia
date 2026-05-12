use std::path::Path;

use crate::io::read_hook_manifest;
use crate::types::{PluginCatalogEntry, PluginHookEntry};

pub fn build_hook_registry(plugins: &[PluginCatalogEntry]) -> Vec<PluginHookEntry> {
  let mut hooks = vec![];

  for plugin in plugins
    .iter()
    .filter(|plugin| plugin.status == "ready" && plugin.enabled)
  {
    let Some(plugin_root) = Path::new(&plugin.manifest_path).parent() else {
      continue;
    };

    for capability in &plugin.capabilities {
      let Some((kind, identifier)) = capability.split_once(':') else {
        continue;
      };
      if kind != "hook" {
        continue;
      }

      let hook_path = plugin_root.join("hooks").join(format!("{identifier}.json"));
      let Ok(hook) = read_hook_manifest(&hook_path) else {
        continue;
      };
      let memory_note_title = hook.memory.as_ref().map(|memory| memory.note_title.clone());
      let memory_note_source = hook
        .memory
        .as_ref()
        .and_then(|memory| memory.note_source.clone());
      let memory_note_tags = hook
        .memory
        .as_ref()
        .map(|memory| memory.note_tags.clone())
        .unwrap_or_default();

      hooks.push(PluginHookEntry {
        hook_id: format!("{}::{}", plugin.id, identifier),
        title: hook.title,
        description: hook.description,
        event: hook.event,
        message_template: hook.message_template,
        plugin_id: plugin.id.clone(),
        plugin_display_name: plugin.display_name.clone(),
        permissions: plugin.permissions.clone(),
        source_path: hook_path.display().to_string(),
        memory_note_title,
        memory_note_source,
        memory_note_tags,
      });
    }
  }

  hooks.sort_by(|left, right| {
    left
      .event
      .cmp(&right.event)
      .then_with(|| left.plugin_display_name.cmp(&right.plugin_display_name))
      .then_with(|| left.title.cmp(&right.title))
      .then_with(|| left.hook_id.cmp(&right.hook_id))
  });
  hooks
}
