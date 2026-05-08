use std::path::Path;

pub(crate) fn infer_requested_file_path(message: &str, workspace_root: &Path) -> Option<String> {
  let common_files = [
    "README.md",
    "Cargo.toml",
    "Package.swift",
    "package.json",
    "pyproject.toml",
  ];
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

    let normalized_candidate = candidate.replace('\\', "/");
    if workspace_root.join(&normalized_candidate).is_file() {
      return Some(normalized_candidate);
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
}
