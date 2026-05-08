pub(crate) fn infer_shell_command(message: &str) -> Option<String> {
  let trimmed = message.trim();
  let lowercased_message = trimmed.to_lowercase();

  for prefix in ["run shell:", "shell:", "run command:"] {
    if lowercased_message.starts_with(prefix) {
      let command = trimmed[prefix.len()..].trim();
      if !command.is_empty() {
        return Some(command.to_string());
      }
    }
  }

  None
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn shell_command_inference_uses_explicit_prefixes() {
    assert_eq!(
      infer_shell_command("run command: git status --short").as_deref(),
      Some("git status --short")
    );
    assert!(infer_shell_command("please run git status").is_none());
  }
}
