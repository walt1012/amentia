pub(crate) fn shell_output_preview(output: &str) -> String {
  let preview = output
    .lines()
    .find(|line| !line.trim().is_empty())
    .unwrap_or(output)
    .trim();

  if preview.is_empty() {
    "none".to_string()
  } else {
    preview.chars().take(120).collect()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn shell_output_preview_uses_first_non_empty_line() {
    assert_eq!(
      shell_output_preview("\n\n  first line\nsecond line"),
      "first line"
    );
    assert_eq!(shell_output_preview("   \n\t"), "none");
  }
}
