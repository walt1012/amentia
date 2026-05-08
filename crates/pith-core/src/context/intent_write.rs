#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WriteIntent {
  pub(crate) relative_path: String,
  pub(crate) content: String,
}

pub(crate) fn infer_write_intent(message: &str) -> Option<WriteIntent> {
  let trimmed = message.trim();
  let lowercased_message = trimmed.to_lowercase();

  for prefix in ["write ", "create ", "update "] {
    if lowercased_message.starts_with(prefix) {
      let remainder = &trimmed[prefix.len()..];
      let (path, content) = remainder.split_once(':')?;
      let relative_path = path
        .trim()
        .trim_matches(&['"', '\'', '`'][..])
        .replace('\\', "/");
      let content = content.trim().to_string();

      if relative_path.is_empty() || content.is_empty() {
        return None;
      }

      return Some(WriteIntent {
        relative_path,
        content,
      });
    }
  }

  None
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
}
