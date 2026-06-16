use std::collections::HashMap;

use pith_plugin_host::PluginCommandEntry as HostPluginCommandEntry;
use pith_protocol::TimelineItem;

use super::plugin_command_runner_attribute_policy::{
  merge_plugin_runner_attributes, plugin_runner_owned_attributes,
};
use super::plugin_command_runner_contracts::{
  PluginRunnerTimelineItemEnvelope, PLUGIN_RUNNER_ALLOWED_TIMELINE_KINDS,
};
use super::plugin_command_runner_proof::{
  insert_plugin_runner_timeline_contracts, plugin_runner_timeline_contracts_are_valid,
};

pub(super) fn plugin_runner_timeline_items(
  command: &HostPluginCommandEntry,
  execution_kind: &str,
  base_attributes: &HashMap<String, String>,
  items: Vec<PluginRunnerTimelineItemEnvelope>,
) -> (Vec<TimelineItem>, usize) {
  let total_item_count = items.len();
  let valid_items = items
    .into_iter()
    .filter_map(|item| plugin_runner_timeline_item(command, execution_kind, base_attributes, item))
    .collect::<Vec<_>>();
  let invalid_item_count = total_item_count.saturating_sub(valid_items.len());

  (valid_items, invalid_item_count)
}

pub(super) fn plugin_runner_timeline_items_with_attributes(
  items: Vec<TimelineItem>,
  attributes: &HashMap<String, String>,
) -> Vec<TimelineItem> {
  items
    .into_iter()
    .map(|mut item| {
      let item_attributes = item.attributes.get_or_insert_with(HashMap::new);
      merge_plugin_runner_attributes(item_attributes, attributes);
      item
    })
    .collect()
}

fn plugin_runner_timeline_item(
  command: &HostPluginCommandEntry,
  execution_kind: &str,
  base_attributes: &HashMap<String, String>,
  item: PluginRunnerTimelineItemEnvelope,
) -> Option<TimelineItem> {
  let kind = item.kind.trim();
  let title = item.title.trim();
  let content = item.content.trim();
  if !plugin_runner_timeline_kind_is_allowed(kind) || title.is_empty() || content.is_empty() {
    return None;
  }

  let mut attributes = plugin_runner_owned_attributes(item.attributes);
  attributes.extend(base_attributes.clone());
  attributes
    .entry("pluginId".to_string())
    .or_insert_with(|| command.plugin_id.clone());
  attributes
    .entry("commandId".to_string())
    .or_insert_with(|| command.command_id.clone());
  attributes
    .entry("executionKind".to_string())
    .or_insert_with(|| execution_kind.to_string());
  attributes
    .entry("sourcePath".to_string())
    .or_insert_with(|| command.source_path.clone());

  if !plugin_runner_timeline_contracts_are_valid(command, &attributes) {
    return None;
  }
  insert_plugin_runner_timeline_contracts(&mut attributes);

  Some(TimelineItem {
    kind: kind.to_string(),
    title: title.to_string(),
    content: content.to_string(),
    attributes: Some(attributes),
  })
}

fn plugin_runner_timeline_kind_is_allowed(kind: &str) -> bool {
  PLUGIN_RUNNER_ALLOWED_TIMELINE_KINDS.contains(&kind)
}
