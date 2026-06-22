use std::collections::{BTreeSet, HashMap};

use amentia_plugin_host::{
  build_skill_registry, PluginCatalogEntry, PluginSkillEntry as HostPluginSkillEntry,
};

const MAX_PLUGIN_SKILL_CONTEXT_SKILLS: usize = 2;
const PLUGIN_SKILL_CONTEXT_BUDGET_CHARS: usize = 900;
const MAX_PLUGIN_SKILL_PREVIEW_CHARS: usize = 320;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct PluginSkillContextPack {
  pub(crate) skills: Vec<PluginSkillContextEntry>,
  pub(crate) candidate_skill_count: usize,
  pub(crate) omitted_skill_count: usize,
  pub(crate) truncated_skill_count: usize,
  pub(crate) estimated_char_count: usize,
  pub(crate) budget_char_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PluginSkillContextEntry {
  pub(crate) skill_id: String,
  pub(crate) plugin_id: String,
  pub(crate) plugin_display_name: String,
  pub(crate) description: String,
  pub(crate) preview: Option<String>,
  pub(crate) score: usize,
}

struct ScoredPluginSkill {
  skill: HostPluginSkillEntry,
  score: usize,
}

pub(crate) fn pack_plugin_skills_for_context(
  plugins: &[PluginCatalogEntry],
  query: &str,
) -> PluginSkillContextPack {
  let mut candidates = build_skill_registry(plugins)
    .into_iter()
    .filter(|skill| skill.status == "ready")
    .filter_map(|skill| {
      let score = skill_match_score(&skill, query);
      (score > 0).then_some(ScoredPluginSkill { skill, score })
    })
    .collect::<Vec<_>>();
  candidates.sort_by(|left, right| {
    right
      .score
      .cmp(&left.score)
      .then_with(|| {
        left
          .skill
          .plugin_display_name
          .cmp(&right.skill.plugin_display_name)
      })
      .then_with(|| left.skill.skill_id.cmp(&right.skill.skill_id))
  });

  let candidate_skill_count = candidates.len();
  let mut pack = PluginSkillContextPack {
    budget_char_count: PLUGIN_SKILL_CONTEXT_BUDGET_CHARS,
    candidate_skill_count,
    ..PluginSkillContextPack::default()
  };

  for candidate in candidates {
    if pack.skills.len() >= MAX_PLUGIN_SKILL_CONTEXT_SKILLS {
      pack.omitted_skill_count += 1;
      continue;
    }

    let (entry, was_truncated) = context_entry(candidate);
    let mut next_pack = pack.clone();
    next_pack.skills.push(entry.clone());
    next_pack.estimated_char_count = format_plugin_skill_context_prompt(&next_pack).len();

    if next_pack.estimated_char_count > next_pack.budget_char_count {
      let Some(smaller_entry) = shrink_entry_to_fit(&pack, entry) else {
        pack.omitted_skill_count += 1;
        continue;
      };
      pack.truncated_skill_count += 1;
      pack.skills.push(smaller_entry);
    } else {
      if was_truncated {
        pack.truncated_skill_count += 1;
      }
      pack.skills.push(entry);
    }

    pack.estimated_char_count = format_plugin_skill_context_prompt(&pack).len();
  }

  pack
}

pub(crate) fn format_plugin_skill_context_prompt(pack: &PluginSkillContextPack) -> String {
  if pack.skills.is_empty() {
    return "Plugin skills: none selected.".to_string();
  }

  let mut lines = vec![format!(
    "Plugin skills: optional local guidance from enabled plugins; \
     use only when relevant and keep the user request higher priority. \
     This is not permission to execute plugin code or contact external services. \
     selected={} candidates={} omitted={} truncated={} budget={}c.",
    pack.skills.len(),
    pack.candidate_skill_count,
    pack.omitted_skill_count,
    pack.truncated_skill_count,
    pack.budget_char_count
  )];
  lines.extend(pack.skills.iter().map(format_plugin_skill_context_line));
  lines.join("\n")
}

pub(crate) fn merge_plugin_skill_context_attributes(
  attributes: &mut HashMap<String, String>,
  pack: &PluginSkillContextPack,
) {
  attributes.insert(
    "pluginSkillContextSelectedCount".to_string(),
    pack.skills.len().to_string(),
  );
  attributes.insert(
    "pluginSkillContextCandidateCount".to_string(),
    pack.candidate_skill_count.to_string(),
  );
  attributes.insert(
    "pluginSkillContextOmittedCount".to_string(),
    pack.omitted_skill_count.to_string(),
  );
  attributes.insert(
    "pluginSkillContextTruncatedCount".to_string(),
    pack.truncated_skill_count.to_string(),
  );
  attributes.insert(
    "pluginSkillContextEstimatedChars".to_string(),
    pack.estimated_char_count.to_string(),
  );
  attributes.insert(
    "pluginSkillContextBudgetChars".to_string(),
    pack.budget_char_count.to_string(),
  );
  attributes.insert(
    "pluginSkillContextRevocable".to_string(),
    "true".to_string(),
  );
  attributes.insert(
    "pluginSkillContextSource".to_string(),
    "enabledPluginSkills".to_string(),
  );
  if !pack.skills.is_empty() {
    attributes.insert(
      "pluginSkillContextSkillIds".to_string(),
      pack
        .skills
        .iter()
        .map(|skill| skill.skill_id.as_str())
        .collect::<Vec<_>>()
        .join(","),
    );
    attributes.insert(
      "pluginSkillContextPluginIds".to_string(),
      joined_unique_attribute(
        pack
          .skills
          .iter()
          .map(|skill| skill.plugin_id.as_str()),
      ),
    );
    attributes.insert(
      "pluginSkillContextPluginNames".to_string(),
      joined_unique_attribute(
        pack
          .skills
          .iter()
          .map(|skill| skill.plugin_display_name.as_str()),
      ),
    );
    attributes.insert(
      "pluginSkillContextSkillDescriptions".to_string(),
      pack
        .skills
        .iter()
        .map(|skill| skill.description.as_str())
        .collect::<Vec<_>>()
        .join(" | "),
    );
  }
}

fn context_entry(candidate: ScoredPluginSkill) -> (PluginSkillContextEntry, bool) {
  let (preview, was_truncated) = candidate
    .skill
    .preview
    .as_deref()
    .map(compact_preview)
    .map(|preview| truncate_chars(&preview, MAX_PLUGIN_SKILL_PREVIEW_CHARS))
    .unwrap_or((None, false));

  (
    PluginSkillContextEntry {
      skill_id: candidate.skill.skill_id,
      plugin_id: candidate.skill.plugin_id,
      plugin_display_name: candidate.skill.plugin_display_name,
      description: candidate.skill.description,
      preview,
      score: candidate.score,
    },
    was_truncated,
  )
}

fn joined_unique_attribute<'a>(values: impl Iterator<Item = &'a str>) -> String {
  values
    .filter(|value| !value.trim().is_empty())
    .collect::<BTreeSet<_>>()
    .into_iter()
    .collect::<Vec<_>>()
    .join(",")
}

fn shrink_entry_to_fit(
  pack: &PluginSkillContextPack,
  mut entry: PluginSkillContextEntry,
) -> Option<PluginSkillContextEntry> {
  let mut preview_budget = entry
    .preview
    .as_ref()
    .map(|preview| preview.len())
    .unwrap_or(0);
  while preview_budget > 0 {
    preview_budget /= 2;
    entry.preview = entry
      .preview
      .as_deref()
      .and_then(|preview| truncate_chars(preview, preview_budget).0);
    let mut next_pack = pack.clone();
    next_pack.skills.push(entry.clone());
    if format_plugin_skill_context_prompt(&next_pack).len() <= next_pack.budget_char_count {
      return Some(entry);
    }
  }

  entry.preview = None;
  let mut next_pack = pack.clone();
  next_pack.skills.push(entry.clone());
  (format_plugin_skill_context_prompt(&next_pack).len() <= next_pack.budget_char_count)
    .then_some(entry)
}

fn format_plugin_skill_context_line(skill: &PluginSkillContextEntry) -> String {
  let mut line = format!(
    "- {}: {} [{} score={}]",
    skill.plugin_display_name, skill.description, skill.skill_id, skill.score
  );
  if let Some(preview) = &skill.preview {
    line.push_str(&format!(" Preview: {preview}"));
  }
  line
}

fn skill_match_score(skill: &HostPluginSkillEntry, query: &str) -> usize {
  let query_tokens = token_set(query);
  if query_tokens.is_empty() {
    return 0;
  }

  let skill_text = format!(
    "{} {} {} {}",
    skill.skill_id,
    skill.plugin_display_name,
    skill.description,
    skill.preview.as_deref().unwrap_or("")
  );
  let skill_tokens = token_set(&skill_text);
  let overlap_score = query_tokens
    .intersection(&skill_tokens)
    .count()
    .saturating_mul(10);
  let plugin_score = normalized_contains(query, &skill.plugin_display_name) as usize * 20;

  overlap_score + plugin_score
}

fn normalized_contains(haystack: &str, needle: &str) -> bool {
  let normalized_haystack = normalize_text(haystack);
  let normalized_needle = normalize_text(needle);
  !normalized_needle.is_empty() && normalized_haystack.contains(&normalized_needle)
}

fn token_set(value: &str) -> BTreeSet<String> {
  normalize_text(value)
    .split_whitespace()
    .filter(|token| token.len() >= 3)
    .filter(|token| !is_common_plugin_context_token(token))
    .map(str::to_string)
    .collect()
}

fn is_common_plugin_context_token(token: &str) -> bool {
  matches!(
    token,
    "and"
      | "are"
      | "before"
      | "can"
      | "for"
      | "from"
      | "into"
      | "local"
      | "plugin"
      | "prepare"
      | "review"
      | "skill"
      | "skills"
      | "that"
      | "the"
      | "this"
      | "use"
      | "when"
      | "with"
      | "workspace"
      | "workspaces"
      | "context"
  )
}

fn normalize_text(value: &str) -> String {
  value
    .chars()
    .map(|character| {
      if character.is_ascii_alphanumeric() {
        character.to_ascii_lowercase()
      } else {
        ' '
      }
    })
    .collect::<String>()
    .split_whitespace()
    .collect::<Vec<_>>()
    .join(" ")
}

fn compact_preview(preview: &str) -> String {
  preview.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn truncate_chars(value: &str, max_chars: usize) -> (Option<String>, bool) {
  if max_chars == 0 {
    return (None, !value.is_empty());
  }
  if value.chars().count() <= max_chars {
    return (Some(value.to_string()), false);
  }
  let truncated = value
    .chars()
    .take(max_chars.saturating_sub(3))
    .collect::<String>();
  (Some(format!("{truncated}...")), true)
}

#[cfg(test)]
mod tests {
  use std::fs;
  use std::path::{Path, PathBuf};
  use std::time::{SystemTime, UNIX_EPOCH};

  use amentia_plugin_host::discover_plugins;

  use super::*;

  #[test]
  fn plugin_skill_context_selects_relevant_ready_enabled_skills() {
    let plugin_root = create_temp_plugin_root("plugin-skill-context");
    write_skill_plugin(
      &plugin_root,
      "notion-connector",
      "Notion Connector",
      true,
      "notion.workspace",
      "Prepare workspace context for Notion drafts.",
      "Use this skill when drafting concise Notion pages from local workspace context.",
    );
    write_skill_plugin(
      &plugin_root,
      "calendar-connector",
      "Calendar Connector",
      true,
      "calendar.events",
      "Review calendar events.",
      "Use this skill for local calendar scheduling context.",
    );
    let plugins = discover_plugins(&plugin_root).expect("discover plugins");

    let context =
      pack_plugin_skills_for_context(&plugins, "prepare a Notion page draft from this workspace");

    fs::remove_dir_all(&plugin_root).expect("cleanup plugin root");

    assert_eq!(context.skills.len(), 1);
    assert_eq!(
      context.skills[0].skill_id,
      "notion-connector::notion.workspace"
    );
    assert_eq!(context.candidate_skill_count, 1);
    assert_eq!(context.omitted_skill_count, 0);

    let prompt = format_plugin_skill_context_prompt(&context);
    assert!(prompt.contains("optional local guidance"));
    assert!(prompt.contains("Notion Connector"));
    assert!(prompt.contains("drafting concise Notion pages"));

    let mut attributes = std::collections::HashMap::new();
    merge_plugin_skill_context_attributes(&mut attributes, &context);
    assert_eq!(attributes["pluginSkillContextSelectedCount"], "1");
    assert_eq!(
      attributes["pluginSkillContextSkillIds"],
      "notion-connector::notion.workspace"
    );
    assert_eq!(
      attributes["pluginSkillContextPluginIds"],
      "notion-connector"
    );
    assert_eq!(
      attributes["pluginSkillContextPluginNames"],
      "Notion Connector"
    );
    assert_eq!(
      attributes["pluginSkillContextSkillDescriptions"],
      "Prepare workspace context for Notion drafts."
    );
    assert_eq!(attributes["pluginSkillContextRevocable"], "true");
  }

  #[test]
  fn plugin_skill_context_stays_bounded_and_ignores_disabled_plugins() {
    let plugin_root = create_temp_plugin_root("plugin-skill-context-budget");
    write_skill_plugin(
      &plugin_root,
      "notion-connector",
      "Notion Connector",
      true,
      "notion.workspace",
      "Prepare Notion workspace drafts.",
      &format!("Use this skill for Notion drafts. {}", "x".repeat(4000)),
    );
    write_skill_plugin(
      &plugin_root,
      "notion-disabled",
      "Disabled Notion",
      false,
      "notion.disabled",
      "Prepare disabled Notion drafts.",
      "This disabled skill must not be selected.",
    );
    let plugins = discover_plugins(&plugin_root).expect("discover plugins");

    let context = pack_plugin_skills_for_context(&plugins, "Notion draft");
    let prompt = format_plugin_skill_context_prompt(&context);

    fs::remove_dir_all(&plugin_root).expect("cleanup plugin root");

    assert_eq!(context.skills.len(), 1);
    assert_eq!(context.skills[0].plugin_display_name, "Notion Connector");
    assert!(context.truncated_skill_count > 0);
    assert!(prompt.len() <= context.budget_char_count);
    assert!(!prompt.contains("Disabled Notion"));
  }

  fn write_skill_plugin(
    plugin_root: &Path,
    plugin_name: &str,
    display_name: &str,
    default_enabled: bool,
    skill_id: &str,
    description: &str,
    body: &str,
  ) {
    let plugin_dir = plugin_root.join(plugin_name);
    let skills_dir = plugin_dir.join("skills");
    fs::create_dir_all(&skills_dir).expect("create skills dir");
    fs::write(
      plugin_dir.join("amentia-plugin.json"),
      format!(
        r#"{{
  "name": "{plugin_name}",
  "version": "0.1.0",
  "displayName": "{display_name}",
  "description": "Skill plugin",
  "author": {{ "name": "Amentia" }},
  "permissions": ["network.outbound"],
  "skills": [
    {{
      "id": "{skill_id}",
      "description": "{description}",
      "path": "skills/main.md"
    }}
  ],
  "defaultEnabled": {default_enabled}
}}"#
      ),
    )
    .expect("write plugin manifest");
    fs::write(skills_dir.join("main.md"), body).expect("write skill body");
  }

  fn create_temp_plugin_root(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
      .duration_since(UNIX_EPOCH)
      .expect("clock")
      .as_nanos();
    let path = std::env::temp_dir().join(format!("amentia-{label}-{nonce}"));
    fs::create_dir_all(&path).expect("create plugin root");
    path
  }
}
