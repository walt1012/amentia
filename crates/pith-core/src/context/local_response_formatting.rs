use pith_tools::{DirectoryEntry, ReadFileResult, SearchMatch, ShellCommandResult};

pub(crate) fn format_file_result(result: &ReadFileResult) -> String {
  if result.is_truncated {
    format!(
      "File: {}\n\n{}\n\n[output truncated at 4096 bytes]",
      result.relative_path, result.content
    )
  } else {
    format!("File: {}\n\n{}", result.relative_path, result.content)
  }
}

pub(crate) fn format_directory_result(entries: &[DirectoryEntry]) -> String {
  if entries.is_empty() {
    return "The directory is empty.".to_string();
  }

  entries
    .iter()
    .map(|entry| format!("[{}] {}", entry.entry_type, entry.relative_path))
    .collect::<Vec<_>>()
    .join("\n")
}

pub(crate) fn format_search_result(query: &str, matches: &[SearchMatch]) -> String {
  if matches.is_empty() {
    return format!("No matches found for \"{}\".", query);
  }

  matches
    .iter()
    .map(|entry| {
      format!(
        "{}:{}: {}",
        entry.relative_path, entry.line_number, entry.line
      )
    })
    .collect::<Vec<_>>()
    .join("\n")
}

pub(crate) fn format_shell_result(result: &ShellCommandResult) -> String {
  let stdout = if result.stdout.trim().is_empty() {
    "[no stdout]".to_string()
  } else {
    result.stdout.clone()
  };
  let stderr = if result.stderr.trim().is_empty() {
    "[no stderr]".to_string()
  } else {
    result.stderr.clone()
  };
  let truncation_note = if result.was_truncated {
    "\n\n[output truncated]"
  } else {
    ""
  };
  let timeout_note = if result.timed_out {
    "\n\n[command timed out]"
  } else {
    ""
  };
  format!(
    "Command: {}\nExit Code: {}\n{}\n\nstdout:\n{}\n\nstderr:\n{}{}{}",
    result.command,
    result.exit_code,
    result.sandbox.display_line(),
    stdout,
    stderr,
    truncation_note,
    timeout_note
  )
}
