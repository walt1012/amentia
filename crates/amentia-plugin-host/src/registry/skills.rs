use std::fs;
use std::path::{Component, Path, PathBuf};

use crate::io::read_manifest;
use crate::types::{PluginCatalogEntry, PluginSkillEntry};

const MAX_SKILL_PREVIEW_CHARS: usize = 1200;

pub fn build_skill_registry(plugins: &[PluginCatalogEntry]) -> Vec<PluginSkillEntry> {
  let mut skills = plugins
    .iter()
    .filter(|plugin| plugin.status == "ready" && plugin.enabled)
    .flat_map(plugin_skill_entries)
    .collect::<Vec<_>>();

  skills.sort_by(|left, right| {
    left
      .plugin_display_name
      .cmp(&right.plugin_display_name)
      .then_with(|| left.skill_id.cmp(&right.skill_id))
  });
  skills
}

fn plugin_skill_entries(plugin: &PluginCatalogEntry) -> Vec<PluginSkillEntry> {
  let manifest_path = Path::new(&plugin.manifest_path);
  let Ok(manifest) = read_manifest(manifest_path) else {
    return vec![];
  };
  let Some(plugin_root) = manifest_path.parent() else {
    return vec![];
  };

  manifest
    .skills
    .into_iter()
    .filter_map(|skill| {
      let source_path = safe_skill_path(plugin_root, &skill.path)?;
      Some(skill_entry(
        plugin,
        &skill.id,
        &skill.description,
        source_path,
      ))
    })
    .collect()
}

fn skill_entry(
  plugin: &PluginCatalogEntry,
  skill_id: &str,
  description: &str,
  source_path: PathBuf,
) -> PluginSkillEntry {
  match fs::read_to_string(&source_path) {
    Ok(content) => PluginSkillEntry {
      skill_id: format!("{}::{skill_id}", plugin.id),
      description: description.to_string(),
      plugin_id: plugin.id.clone(),
      plugin_display_name: plugin.display_name.clone(),
      permissions: plugin.permissions.clone(),
      source_path: source_path.display().to_string(),
      status: "ready".to_string(),
      preview: Some(skill_preview(&content)),
      content_bytes: content.len(),
      run_blocker: None,
      run_repair_hint: None,
    },
    Err(error) => PluginSkillEntry {
      skill_id: format!("{}::{skill_id}", plugin.id),
      description: description.to_string(),
      plugin_id: plugin.id.clone(),
      plugin_display_name: plugin.display_name.clone(),
      permissions: plugin.permissions.clone(),
      source_path: source_path.display().to_string(),
      status: if source_path.exists() {
        "invalidSkillFile".to_string()
      } else {
        "missingSkillFile".to_string()
      },
      preview: None,
      content_bytes: 0,
      run_blocker: Some(format!("Skill file could not be loaded: {error}")),
      run_repair_hint: Some(
        "Check the skill file inside the plugin bundle, then refresh plugins.".to_string(),
      ),
    },
  }
}

fn skill_preview(content: &str) -> String {
  content
    .chars()
    .take(MAX_SKILL_PREVIEW_CHARS)
    .collect::<String>()
    .trim()
    .to_string()
}

fn safe_skill_path(plugin_root: &Path, relative_path: &str) -> Option<PathBuf> {
  let trimmed = relative_path.trim();
  if trimmed.is_empty()
    || trimmed.contains('\\')
    || trimmed.contains(':')
    || Path::new(trimmed).is_absolute()
  {
    return None;
  }
  if !Path::new(trimmed)
    .components()
    .all(|component| matches!(component, Component::Normal(_)))
  {
    return None;
  }
  Some(plugin_root.join(trimmed))
}
