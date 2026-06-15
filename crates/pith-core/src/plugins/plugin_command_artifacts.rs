use std::path::Path;

use pith_protocol::WorkspaceSummary;
use pith_tools::read_file;

use super::plugin_command_types::PluginConnectorExecutionRef;

const CONNECTOR_SAVED_ARTIFACT_PREFIX: &str = "Saved artifact: ";
const CONNECTOR_SAVED_ARTIFACT_PREVIEW_PREFIX: &str = "Saved artifact preview: ";
const CONNECTOR_SAVED_ARTIFACT_TRUNCATED_PREFIX: &str = "Saved artifact truncated: ";
const CONNECTOR_SAVED_ARTIFACT_READ_ERROR_PREFIX: &str = "Saved artifact read error: ";
const CONNECTOR_SAVED_ARTIFACT_MAX_LEN: usize = 240;
const CONNECTOR_SAVED_ARTIFACT_PREVIEW_MAX_BYTES: usize = 2048;
const CONNECTOR_SAVED_ARTIFACT_PREVIEW_MAX_CHARS: usize = 700;

pub(crate) fn connector_command_input(message: &str) -> Option<String> {
  let trimmed = message.trim();
  if trimmed.is_empty() {
    return None;
  }
  if trimmed
    .to_ascii_lowercase()
    .contains(&CONNECTOR_SAVED_ARTIFACT_PREFIX.to_ascii_lowercase())
  {
    return Some(trimmed.to_string());
  }

  extract_saved_artifact_reference(trimmed)
    .map(|path| format!("{trimmed}\n\n{CONNECTOR_SAVED_ARTIFACT_PREFIX}{path}"))
    .or_else(|| Some(trimmed.to_string()))
}

pub(super) fn expand_connector_saved_artifact_input(
  workspace: Option<&WorkspaceSummary>,
  input: Option<String>,
  connector_refs: &[PluginConnectorExecutionRef],
) -> Option<String> {
  if connector_refs.is_empty() {
    return input;
  }
  let input = input?;
  let normalized_input = input.to_ascii_lowercase();
  if normalized_input.contains(&CONNECTOR_SAVED_ARTIFACT_PREVIEW_PREFIX.to_ascii_lowercase())
    || normalized_input.contains(&CONNECTOR_SAVED_ARTIFACT_READ_ERROR_PREFIX.to_ascii_lowercase())
  {
    return Some(input);
  }
  let Some(path) = extract_saved_artifact_reference(&input) else {
    return Some(input);
  };
  let artifact_context = match workspace {
    Some(workspace) => bounded_artifact_preview(workspace, &path),
    None => format!(
      "{CONNECTOR_SAVED_ARTIFACT_READ_ERROR_PREFIX}Open a project before using saved artifacts."
    ),
  };

  Some(format!("{input}\n{artifact_context}"))
}

fn bounded_artifact_preview(workspace: &WorkspaceSummary, relative_path: &str) -> String {
  match read_file(
    Path::new(&workspace.root_path),
    relative_path,
    CONNECTOR_SAVED_ARTIFACT_PREVIEW_MAX_BYTES,
  ) {
    Ok(result) => format!(
      "{CONNECTOR_SAVED_ARTIFACT_PREVIEW_PREFIX}{}\n{CONNECTOR_SAVED_ARTIFACT_TRUNCATED_PREFIX}{}",
      compact_artifact_preview(&result.content),
      result.is_truncated
    ),
    Err(error) => format!(
      "{CONNECTOR_SAVED_ARTIFACT_READ_ERROR_PREFIX}{}",
      compact_artifact_preview(&error.to_string())
    ),
  }
}

fn extract_saved_artifact_reference(message: &str) -> Option<String> {
  message
    .split_whitespace()
    .filter_map(saved_artifact_candidate)
    .find(|candidate| is_saved_artifact_reference(candidate))
}

fn saved_artifact_candidate(raw: &str) -> Option<String> {
  let trimmed = raw
    .trim_matches(saved_artifact_edge_punctuation)
    .trim_end_matches(saved_artifact_trailing_punctuation)
    .replace('\\', "/");
  (!trimmed.is_empty()).then_some(trimmed)
}

fn saved_artifact_edge_punctuation(character: char) -> bool {
  matches!(
    character,
    '"' | '\'' | '`' | '(' | ')' | '[' | ']' | '{' | '}' | '<' | '>' | ',' | ';' | ':'
  )
}

fn saved_artifact_trailing_punctuation(character: char) -> bool {
  matches!(character, '.' | '!' | '?')
}

fn is_saved_artifact_reference(candidate: &str) -> bool {
  if candidate.len() > CONNECTOR_SAVED_ARTIFACT_MAX_LEN
    || candidate.starts_with('/')
    || candidate.starts_with('~')
    || candidate.contains("://")
    || candidate.contains(':')
  {
    return false;
  }
  if candidate.chars().any(|character| {
    !character.is_ascii_alphanumeric() && !matches!(character, '/' | '.' | '_' | '-')
  }) {
    return false;
  }
  if candidate
    .split('/')
    .any(|segment| segment.is_empty() || segment == "." || segment == "..")
  {
    return false;
  }

  let normalized = candidate.to_ascii_lowercase();
  let supported_extension = [".md", ".txt", ".json", ".yaml", ".yml", ".toml", ".csv"]
    .iter()
    .any(|extension| normalized.ends_with(extension));
  if !supported_extension {
    return false;
  }

  candidate.contains('/') || looks_like_saved_artifact_name(&normalized)
}

fn looks_like_saved_artifact_name(normalized: &str) -> bool {
  [
    "artifact", "brief", "handoff", "note", "notes", "plan", "review", "summary", "update",
  ]
  .iter()
  .any(|term| normalized.contains(term))
}

fn compact_artifact_preview(content: &str) -> String {
  let compacted = content
    .chars()
    .map(|character| match character {
      '\r' | '\n' | '\t' => ' ',
      '"' | '\\' => '\'',
      character if character.is_control() => ' ',
      character => character,
    })
    .collect::<String>()
    .split_whitespace()
    .collect::<Vec<_>>()
    .join(" ");
  let truncated = compacted
    .chars()
    .take(CONNECTOR_SAVED_ARTIFACT_PREVIEW_MAX_CHARS)
    .collect::<String>();

  if truncated.is_empty() {
    "(empty file)".to_string()
  } else {
    truncated
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn connector_input_marks_saved_artifact_reference() {
    let input = connector_command_input("Prepare a Notion update from docs/handoff.md.")
      .expect("connector input");

    assert_eq!(
      input,
      "Prepare a Notion update from docs/handoff.md.\n\nSaved artifact: docs/handoff.md"
    );
  }

  #[test]
  fn saved_artifact_reference_rejects_urls_and_traversal() {
    assert!(extract_saved_artifact_reference("Use https://example.com/handoff.md").is_none());
    assert!(extract_saved_artifact_reference("Use ../handoff.md").is_none());
    assert!(extract_saved_artifact_reference("Use C:\\temp\\handoff.md").is_none());
    assert_eq!(
      extract_saved_artifact_reference("Use handoff.md for the connector").as_deref(),
      Some("handoff.md")
    );
  }

  #[test]
  fn compact_preview_is_single_line_and_json_safe() {
    assert_eq!(
      compact_artifact_preview("Title\n\nUse \"quotes\" and \\ slashes."),
      "Title Use 'quotes' and ' slashes."
    );
  }
}
