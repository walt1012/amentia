pub(super) fn compact_text_preview(content: &str, max_lines: usize, max_chars: usize) -> String {
  let mut preview = content
    .lines()
    .map(str::trim)
    .filter(|line| !line.is_empty())
    .take(max_lines)
    .collect::<Vec<_>>()
    .join("\n");
  if preview.is_empty() {
    preview = "No content available.".to_string();
  }
  if preview.chars().count() > max_chars {
    preview = preview.chars().take(max_chars).collect::<String>();
    preview.push_str("...");
  }
  preview
}
