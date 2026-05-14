#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExplicitPluginCommandRoute {
  pub(crate) command_id: String,
  pub(crate) input: Option<String>,
  pub(crate) routing_reason: &'static str,
}

pub(crate) fn infer_explicit_plugin_command_route(
  message: &str,
) -> Option<ExplicitPluginCommandRoute> {
  let trimmed = message.trim();
  for (prefix, routing_reason) in explicit_plugin_prefixes() {
    let Some(remainder) = strip_case_insensitive_prefix(trimmed, prefix) else {
      continue;
    };
    let Some((command_id, input)) = split_command_route(remainder) else {
      continue;
    };
    if !is_supported_command_id(command_id) {
      continue;
    }

    return Some(ExplicitPluginCommandRoute {
      command_id: command_id.to_string(),
      input: input.map(str::to_string),
      routing_reason,
    });
  }

  None
}

fn explicit_plugin_prefixes() -> [(&'static str, &'static str); 5] {
  [
    ("/plugin ", "slashPluginCommand"),
    ("run plugin command ", "explicitPluginCommand"),
    ("run plugin ", "explicitPluginCommand"),
    ("use plugin command ", "explicitPluginCommand"),
    ("use plugin ", "explicitPluginCommand"),
  ]
}

fn strip_case_insensitive_prefix<'a>(value: &'a str, prefix: &str) -> Option<&'a str> {
  value
    .get(..prefix.len())
    .filter(|candidate| candidate.eq_ignore_ascii_case(prefix))
    .map(|_| value[prefix.len()..].trim())
}

fn split_command_route(remainder: &str) -> Option<(&str, Option<&str>)> {
  let trimmed = remainder.trim();
  if trimmed.is_empty() {
    return None;
  }

  let split_at = trimmed
    .find(char::is_whitespace)
    .unwrap_or(trimmed.len());
  let command_id = &trimmed[..split_at];
  let input = trimmed[split_at..].trim();
  Some((
    command_id,
    (!input.is_empty()).then_some(input),
  ))
}

fn is_supported_command_id(command_id: &str) -> bool {
  command_id.contains("::")
    && !command_id.starts_with("::")
    && !command_id.ends_with("::")
    && command_id.chars().all(|character| {
      character.is_ascii_alphanumeric()
        || matches!(character, ':' | '.' | '_' | '-')
    })
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn detects_slash_plugin_command_with_input() {
    let route = infer_explicit_plugin_command_route(
      "/plugin notion-runner::notion-runner.sync sync today",
    )
    .expect("route");

    assert_eq!(route.command_id, "notion-runner::notion-runner.sync");
    assert_eq!(route.input.as_deref(), Some("sync today"));
    assert_eq!(route.routing_reason, "slashPluginCommand");
  }

  #[test]
  fn detects_case_insensitive_run_plugin_command() {
    let route = infer_explicit_plugin_command_route(
      "Run Plugin Command workspace-notes::workspace.capture-note",
    )
    .expect("route");

    assert_eq!(route.command_id, "workspace-notes::workspace.capture-note");
    assert_eq!(route.input, None);
    assert_eq!(route.routing_reason, "explicitPluginCommand");
  }

  #[test]
  fn ignores_natural_language_plugin_mentions() {
    assert!(infer_explicit_plugin_command_route(
      "Can a notion plugin help this thread?"
    )
    .is_none());
  }

  #[test]
  fn ignores_invalid_command_ids() {
    assert!(infer_explicit_plugin_command_route("/plugin missing").is_none());
    assert!(infer_explicit_plugin_command_route("/plugin bad command").is_none());
  }
}
