use std::fs::File;
use std::io::Read;
use std::path::Path;

use anyhow::{Context, Result};

pub(crate) struct BoundedTextPreview {
  pub(crate) content: String,
  pub(crate) is_truncated: bool,
}

pub(crate) fn read_text_prefix(path: &Path, max_bytes: usize) -> Result<BoundedTextPreview> {
  let mut file =
    File::open(path).with_context(|| format!("failed to open file {}", path.display()))?;
  let read_limit = max_bytes.saturating_add(1);
  let mut bytes = Vec::with_capacity(read_limit.min(64 * 1024));
  let mut buffer = [0_u8; 8192];

  while bytes.len() < read_limit {
    let remaining = read_limit.saturating_sub(bytes.len());
    let read_size = remaining.min(buffer.len());
    let bytes_read = file
      .read(&mut buffer[..read_size])
      .with_context(|| format!("failed to read file {}", path.display()))?;
    if bytes_read == 0 {
      break;
    }
    bytes.extend_from_slice(&buffer[..bytes_read]);
  }

  let is_truncated = bytes.len() > max_bytes;
  if is_truncated {
    bytes.truncate(max_bytes);
  }

  Ok(BoundedTextPreview {
    content: String::from_utf8_lossy(&bytes).into_owned(),
    is_truncated,
  })
}

pub(crate) fn text_prefix(content: &str, max_bytes: usize) -> BoundedTextPreview {
  if content.len() <= max_bytes {
    return BoundedTextPreview {
      content: content.to_string(),
      is_truncated: false,
    };
  }

  let mut end = max_bytes;
  while !content.is_char_boundary(end) {
    end = end.saturating_sub(1);
  }

  BoundedTextPreview {
    content: content[..end].to_string(),
    is_truncated: true,
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn text_prefix_preserves_character_boundaries() {
    let preview = text_prefix("abc\u{00e9}def", 4);

    assert_eq!(preview.content, "abc");
    assert!(preview.is_truncated);
  }
}
