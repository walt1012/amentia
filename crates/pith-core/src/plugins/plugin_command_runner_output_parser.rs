use super::plugin_command_runner_contracts::PluginRunnerOutputEnvelope;

const PLUGIN_RUNNER_LOG_PREVIEW_LIMIT: usize = 2048;

pub(super) enum PluginRunnerParsedOutput {
  Envelope(PluginRunnerOutputEnvelope),
  PlainText(String),
  MalformedJson {
    parse_error: String,
    parse_error_preview: String,
  },
}

pub(super) fn parse_plugin_runner_output(output: &str) -> PluginRunnerParsedOutput {
  match serde_json::from_str::<PluginRunnerOutputEnvelope>(output) {
    Ok(envelope) => PluginRunnerParsedOutput::Envelope(envelope),
    Err(error) if plugin_runner_output_looks_like_json(output) => {
      let parse_error = error.to_string();
      PluginRunnerParsedOutput::MalformedJson {
        parse_error_preview: bounded_log_preview(&parse_error),
        parse_error,
      }
    }
    Err(_) => PluginRunnerParsedOutput::PlainText(plugin_runner_plain_text_content(output)),
  }
}

pub(super) fn bounded_log_preview(content: &str) -> String {
  let mut preview = content
    .chars()
    .take(PLUGIN_RUNNER_LOG_PREVIEW_LIMIT)
    .collect::<String>();
  if content.chars().count() > PLUGIN_RUNNER_LOG_PREVIEW_LIMIT {
    preview.push_str("\n[truncated]");
  }
  preview
}

fn plugin_runner_plain_text_content(output: &str) -> String {
  if output.trim().is_empty() {
    return "Plugin command completed without output.".to_string();
  }

  output.to_string()
}

fn plugin_runner_output_looks_like_json(output: &str) -> bool {
  let trimmed = output.trim_start();
  trimmed.starts_with('{') || trimmed.starts_with('[')
}

#[cfg(test)]
mod tests {
  use super::{parse_plugin_runner_output, PluginRunnerParsedOutput};

  #[test]
  fn parses_valid_output_envelope() {
    match parse_plugin_runner_output(r#"{"content":"Saved."}"#) {
      PluginRunnerParsedOutput::Envelope(envelope) => {
        assert_eq!(envelope.content.as_deref(), Some("Saved."));
      }
      _ => panic!("expected output envelope"),
    }
  }

  #[test]
  fn treats_non_json_output_as_plain_text() {
    match parse_plugin_runner_output("Saved.") {
      PluginRunnerParsedOutput::PlainText(content) => assert_eq!(content, "Saved."),
      _ => panic!("expected plain text"),
    }
  }

  #[test]
  fn treats_json_shaped_parse_failures_as_malformed_json() {
    match parse_plugin_runner_output(r#"{"content":42}"#) {
      PluginRunnerParsedOutput::MalformedJson {
        parse_error,
        parse_error_preview,
      } => {
        assert!(parse_error.contains("invalid type"));
        assert_eq!(parse_error_preview, parse_error);
      }
      _ => panic!("expected malformed JSON"),
    }
  }

  #[test]
  fn normalizes_empty_plain_text_output() {
    match parse_plugin_runner_output("   ") {
      PluginRunnerParsedOutput::PlainText(content) => {
        assert_eq!(content, "Plugin command completed without output.");
      }
      _ => panic!("expected plain text"),
    }
  }
}
