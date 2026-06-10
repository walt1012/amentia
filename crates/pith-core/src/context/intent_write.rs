#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WriteIntent {
  pub(crate) relative_path: String,
  pub(crate) content: String,
}

pub(crate) fn infer_write_intent(message: &str) -> Option<WriteIntent> {
  let trimmed = message.trim();
  let lowercased_message = trimmed.to_ascii_lowercase();

  for prefix in ["write ", "create ", "update "] {
    if lowercased_message.starts_with(prefix) {
      let remainder = &trimmed[prefix.len()..];
      return parse_path_content_write(remainder);
    }
  }

  infer_saved_artifact_write_intent(trimmed, &lowercased_message)
}

fn infer_saved_artifact_write_intent(
  trimmed: &str,
  lowercased_message: &str,
) -> Option<WriteIntent> {
  for prefix in ["save ", "store ", "record "] {
    if !lowercased_message.starts_with(prefix) {
      continue;
    }
    let remainder = &trimmed[prefix.len()..];
    let lowercased_remainder = &lowercased_message[prefix.len()..];
    let Some((artifact, path_and_content)) =
      split_saved_artifact_target(remainder, lowercased_remainder)
    else {
      continue;
    };
    if !looks_like_saved_workspace_artifact(artifact) {
      continue;
    }
    return parse_path_content_write(path_and_content);
  }

  None
}

fn split_saved_artifact_target<'a>(
  remainder: &'a str,
  lowercased_remainder: &str,
) -> Option<(&'a str, &'a str)> {
  for marker in [" to ", " in "] {
    if let Some(marker_start) = lowercased_remainder.find(marker) {
      let artifact = remainder[..marker_start].trim();
      let path_and_content = remainder[marker_start + marker.len()..].trim();
      if !artifact.is_empty() && !path_and_content.is_empty() {
        return Some((artifact, path_and_content));
      }
    }
  }

  None
}

fn looks_like_saved_workspace_artifact(artifact: &str) -> bool {
  let normalized = artifact.to_ascii_lowercase();
  [
    "handoff", "note", "notes", "summary", "review", "plan", "brief", "update",
  ]
  .iter()
  .any(|term| normalized.contains(term))
}

fn parse_path_content_write(value: &str) -> Option<WriteIntent> {
  let (path, content) = value.split_once(':')?;
  let relative_path = path
    .trim()
    .trim_matches(&['"', '\'', '`'][..])
    .replace('\\', "/");
  let content = content.trim().to_string();

  if relative_path.is_empty() || content.is_empty() {
    return None;
  }

  Some(WriteIntent {
    relative_path,
    content,
  })
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn write_intent_requires_path_and_content() {
    assert_eq!(
      infer_write_intent("create src/main.rs: fn main() {}"),
      Some(WriteIntent {
        relative_path: "src/main.rs".to_string(),
        content: "fn main() {}".to_string(),
      })
    );
    assert!(infer_write_intent("create src/main.rs:").is_none());
  }

  #[test]
  fn write_intent_accepts_natural_saved_artifacts() {
    assert_eq!(
      infer_write_intent("Save handoff to docs/handoff.md: Ship M7 carefully."),
      Some(WriteIntent {
        relative_path: "docs/handoff.md".to_string(),
        content: "Ship M7 carefully.".to_string(),
      })
    );
    assert_eq!(
      infer_write_intent("Record project note in notes/today.md: Keep Pith small."),
      Some(WriteIntent {
        relative_path: "notes/today.md".to_string(),
        content: "Keep Pith small.".to_string(),
      })
    );
  }

  #[test]
  fn write_intent_ignores_ambiguous_save_requests() {
    assert!(infer_write_intent("Save this thought for later").is_none());
    assert!(infer_write_intent("Save random thing to docs/out.md: nope").is_none());
  }
}
