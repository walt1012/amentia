use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WriteIntent {
  pub(crate) relative_path: String,
  pub(crate) content: String,
}

pub(crate) fn infer_requested_file_path(message: &str, workspace_root: &Path) -> Option<String> {
  let common_files = ["README.md", "Cargo.toml", "Package.swift"];
  let lowercased_message = message.to_lowercase();

  for candidate in common_files {
    if lowercased_message.contains(&candidate.to_lowercase())
      && workspace_root.join(candidate).is_file()
    {
      return Some(candidate.to_string());
    }
  }

  let punctuation: &[char] = &['`', '"', '\'', ',', ';', ':', '(', ')', '[', ']', '{', '}'];
  for token in message.split_whitespace() {
    let candidate = token.trim_matches(punctuation);
    if candidate.is_empty() || (!candidate.contains('/') && !candidate.contains('.')) {
      continue;
    }

    if workspace_root.join(candidate).is_file() {
      return Some(candidate.replace('\\', "/"));
    }
  }

  None
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

pub(crate) fn infer_search_query(message: &str) -> Option<String> {
  let trimmed = message.trim();
  let lowercased_message = trimmed.to_lowercase();

  for keyword in ["search for ", "find ", "search "] {
    if let Some(index) = lowercased_message.find(keyword) {
      let query = trimmed[index + keyword.len()..]
        .trim()
        .trim_matches(&['"', '\'', '.', '?', '!', '`'][..]);
      if !query.is_empty() {
        return Some(query.to_string());
      }
    }
  }

  if lowercased_message.contains("grep ") {
    let query = trimmed
      .split_once("grep ")
      .map(|(_, remainder)| remainder.trim())
      .unwrap_or_default()
      .trim_matches(&['"', '\'', '.', '?', '!', '`'][..]);
    if !query.is_empty() {
      return Some(query.to_string());
    }
  }

  None
}

#[cfg(test)]
mod tests {
  use std::fs;
  use std::time::{SystemTime, UNIX_EPOCH};

  use super::*;

  fn temp_workspace(name: &str) -> std::path::PathBuf {
    let nonce = SystemTime::now()
      .duration_since(UNIX_EPOCH)
      .expect("clock")
      .as_nanos();
    let path = std::env::temp_dir().join(format!("pith-intent-{name}-{nonce}"));
    fs::create_dir_all(&path).expect("create temp workspace");
    path
  }

  #[test]
  fn requested_file_path_prefers_existing_common_files() {
    let workspace = temp_workspace("common-file");
    fs::write(workspace.join("README.md"), "hello").expect("write readme");

    let result = infer_requested_file_path("Please read the README.md", &workspace);

    fs::remove_dir_all(&workspace).expect("cleanup workspace");
    assert_eq!(result.as_deref(), Some("README.md"));
  }

  #[test]
  fn requested_file_path_normalizes_backslashes() {
    let workspace = temp_workspace("path-normalize");
    fs::create_dir_all(workspace.join("src")).expect("create src");
    fs::write(workspace.join("src/lib.rs"), "mod tests;").expect("write file");

    let result = infer_requested_file_path("open `src\\lib.rs`", &workspace);

    fs::remove_dir_all(&workspace).expect("cleanup workspace");
    assert_eq!(result.as_deref(), Some("src/lib.rs"));
  }

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
  fn shell_command_inference_uses_explicit_prefixes() {
    assert_eq!(
      infer_shell_command("run command: git status --short").as_deref(),
      Some("git status --short")
    );
    assert!(infer_shell_command("please run git status").is_none());
  }

  #[test]
  fn search_query_inference_trims_wrapping_punctuation() {
    assert_eq!(
      infer_search_query("search for `RuntimeContext`?").as_deref(),
      Some("RuntimeContext")
    );
    assert_eq!(
      infer_search_query("grep model_runtime.").as_deref(),
      Some("model_runtime")
    );
  }
}
