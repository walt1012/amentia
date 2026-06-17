pub(crate) fn render_hook_message(template: &str, replacements: &[(&str, String)]) -> String {
  let mut rendered = template.to_string();
  for (key, value) in replacements {
    rendered = rendered.replace(&format!("{{{{{key}}}}}"), value);
  }
  rendered
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn hook_message_renderer_replaces_declared_tokens() {
    let rendered = render_hook_message(
      "{{workspaceName}} ran {{command}}",
      &[
        ("workspaceName", "amentia".to_string()),
        ("command", "git status".to_string()),
      ],
    );

    assert_eq!(rendered, "amentia ran git status");
  }
}
