use std::path::Path;

use amentia_tools::DirectoryEntry;

const ENTRY_POINT_CANDIDATES: &[&str] = &[
  "README.md",
  "README",
  "README.txt",
  "AGENTS.md",
  "CLAUDE.md",
  "Cargo.toml",
  "Package.swift",
  "package.json",
  "pyproject.toml",
];

const FOLLOW_UP_MANIFEST_CANDIDATES: &[&str] = &[
  "Cargo.toml",
  "Package.swift",
  "package.json",
  "pyproject.toml",
  "go.mod",
  "deno.json",
  "pnpm-workspace.yaml",
];

pub(super) fn preferred_entry_point(entries: &[DirectoryEntry], message: &str) -> Option<String> {
  if !wants_workspace_overview(message) {
    return None;
  }

  ENTRY_POINT_CANDIDATES.iter().find_map(|candidate| {
    entries
      .iter()
      .find(|entry| {
        entry.entry_type == "file" && entry.relative_path.eq_ignore_ascii_case(candidate)
      })
      .map(|entry| entry.relative_path.clone())
  })
}

pub(super) fn follow_up_manifest_after_entry_point(
  workspace_root: &str,
  current_relative_path: &str,
  message: &str,
) -> Option<String> {
  if !wants_workspace_overview(message) || !is_entry_point(current_relative_path) {
    return None;
  }

  FOLLOW_UP_MANIFEST_CANDIDATES
    .iter()
    .filter(|candidate| !current_relative_path.eq_ignore_ascii_case(candidate))
    .find(|candidate| Path::new(workspace_root).join(candidate).is_file())
    .map(|candidate| candidate.to_string())
}

fn wants_workspace_overview(message: &str) -> bool {
  let lowercased = message.to_lowercase();
  let wants_overview = [
    "analyze",
    "explain",
    "inspect",
    "overview",
    "review",
    "summarize",
    "understand",
    "what is",
    "what's",
  ]
  .iter()
  .any(|signal| lowercased.contains(signal));
  let list_only = ["list", "show files", "show the files"]
    .iter()
    .any(|signal| lowercased.contains(signal));

  wants_overview && !list_only
}

fn is_entry_point(relative_path: &str) -> bool {
  [
    "README.md",
    "README",
    "README.txt",
    "AGENTS.md",
    "CLAUDE.md",
  ]
  .iter()
  .any(|candidate| relative_path.eq_ignore_ascii_case(candidate))
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn overview_requests_choose_readme_before_manifest() {
    let entries = vec![
      entry("Cargo.toml"),
      entry("README.md"),
      entry("package.json"),
    ];

    assert_eq!(
      preferred_entry_point(&entries, "Explain this project").as_deref(),
      Some("README.md")
    );
  }

  #[test]
  fn list_only_requests_do_not_choose_entry_point() {
    let entries = vec![entry("README.md")];

    assert!(preferred_entry_point(&entries, "List the workspace files").is_none());
  }

  fn entry(relative_path: &str) -> DirectoryEntry {
    DirectoryEntry {
      name: relative_path.to_string(),
      relative_path: relative_path.to_string(),
      entry_type: "file".to_string(),
    }
  }
}
