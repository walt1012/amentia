use std::process::Command;

use anyhow::{bail, Context, Result};

use crate::types::WebSearchResult;

const WEB_SEARCH_PROVIDER: &str = "DuckDuckGo Lite";
const WEB_SEARCH_ENDPOINT: &str = "https://lite.duckduckgo.com/lite/";
const WEB_SEARCH_TIMEOUT_SECONDS: &str = "15";
const WEB_SEARCH_CONNECT_TIMEOUT_SECONDS: &str = "8";
const WEB_SEARCH_MAX_BYTES: &str = "1048576";

pub fn web_search(query: &str, max_results: usize) -> Result<Vec<WebSearchResult>> {
  let trimmed_query = query.trim();
  if trimmed_query.is_empty() {
    bail!("web search query must not be empty");
  }
  if max_results == 0 {
    return Ok(vec![]);
  }

  let url = format!(
    "{}?q={}",
    WEB_SEARCH_ENDPOINT,
    percent_encode(trimmed_query)
  );
  let output = Command::new("curl")
    .args([
      "--silent",
      "--show-error",
      "--location",
      "--max-time",
      WEB_SEARCH_TIMEOUT_SECONDS,
      "--connect-timeout",
      WEB_SEARCH_CONNECT_TIMEOUT_SECONDS,
      "--max-filesize",
      WEB_SEARCH_MAX_BYTES,
      "--user-agent",
      "Pith/0.1",
      "--url",
      &url,
    ])
    .output()
    .with_context(|| "failed to start curl for web search")?;
  if !output.status.success() {
    let stderr = String::from_utf8_lossy(&output.stderr);
    bail!("web search request failed: {}", stderr.trim());
  }

  let html = String::from_utf8_lossy(&output.stdout);
  Ok(parse_duckduckgo_lite_results(&html, max_results))
}

fn parse_duckduckgo_lite_results(html: &str, max_results: usize) -> Vec<WebSearchResult> {
  let mut results = Vec::new();
  let mut cursor = 0;
  while results.len() < max_results {
    let Some(class_offset) = html[cursor..].find("result-link") else {
      break;
    };
    let class_index = cursor + class_offset;
    let Some(anchor_start) = html[..class_index].rfind("<a ") else {
      cursor = class_index + "result-link".len();
      continue;
    };
    let Some(anchor_end_offset) = html[class_index..].find("</a>") else {
      break;
    };
    let anchor_end = class_index + anchor_end_offset;
    let anchor = &html[anchor_start..anchor_end];
    let Some(raw_href) = attribute_value(anchor, "href") else {
      cursor = anchor_end + "</a>".len();
      continue;
    };
    let Some(title_start_offset) = anchor.find('>') else {
      cursor = anchor_end + "</a>".len();
      continue;
    };
    let title = clean_html_text(&anchor[title_start_offset + 1..]);
    let url = normalize_result_url(&raw_href);
    if title.is_empty() || url.is_empty() {
      cursor = anchor_end + "</a>".len();
      continue;
    }

    let next_cursor = anchor_end + "</a>".len();
    let snippet = find_result_snippet(&html[next_cursor..]);
    results.push(WebSearchResult {
      title,
      url,
      snippet,
      source: WEB_SEARCH_PROVIDER.to_string(),
    });
    cursor = next_cursor;
  }

  results
}

fn find_result_snippet(html: &str) -> String {
  let result_block = html
    .find("result-link")
    .map(|next_result| &html[..next_result])
    .unwrap_or(html);
  let Some(class_offset) = result_block.find("result-snippet") else {
    return String::new();
  };
  let Some(cell_start_offset) = result_block[..class_offset].rfind("<td") else {
    return String::new();
  };
  let cell_start = cell_start_offset;
  let Some(content_start_offset) = result_block[cell_start..].find('>') else {
    return String::new();
  };
  let content_start = cell_start + content_start_offset + 1;
  let Some(content_end_offset) = result_block[content_start..].find("</td>") else {
    return String::new();
  };

  clean_html_text(&result_block[content_start..content_start + content_end_offset])
}

fn attribute_value(tag: &str, name: &str) -> Option<String> {
  for quote in ['"', '\''] {
    let marker = format!("{name}={quote}");
    if let Some(start) = tag.find(&marker).map(|index| index + marker.len()) {
      let end = tag[start..].find(quote)? + start;
      return Some(html_decode(&tag[start..end]));
    }
  }

  None
}

fn normalize_result_url(raw_href: &str) -> String {
  let decoded_href = html_decode(raw_href);
  let href = if let Some(url) = decoded_href.strip_prefix("//") {
    format!("https:{url}")
  } else {
    decoded_href
  };
  if let Some(parameter_start) = href.find("uddg=") {
    let encoded = &href[parameter_start + "uddg=".len()..];
    let encoded = encoded.split('&').next().unwrap_or(encoded);
    return percent_decode(encoded);
  }

  href
}

fn clean_html_text(value: &str) -> String {
  let mut text = String::new();
  let mut in_tag = false;
  for character in value.chars() {
    match character {
      '<' => in_tag = true,
      '>' => in_tag = false,
      _ if !in_tag => text.push(character),
      _ => {}
    }
  }

  html_decode(&text)
    .split_whitespace()
    .collect::<Vec<_>>()
    .join(" ")
}

fn percent_encode(value: &str) -> String {
  let mut encoded = String::new();
  for byte in value.as_bytes() {
    match *byte {
      b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
        encoded.push(*byte as char);
      }
      b' ' => encoded.push('+'),
      byte => encoded.push_str(&format!("%{byte:02X}")),
    }
  }

  encoded
}

fn percent_decode(value: &str) -> String {
  let mut bytes = Vec::with_capacity(value.len());
  let mut input = value.as_bytes().iter().copied().peekable();
  while let Some(byte) = input.next() {
    if byte == b'+' {
      bytes.push(b' ');
      continue;
    }
    if byte == b'%' {
      let Some(high) = input.next().and_then(hex_value) else {
        bytes.push(byte);
        continue;
      };
      let Some(low) = input.next().and_then(hex_value) else {
        bytes.push(byte);
        continue;
      };
      bytes.push((high << 4) | low);
      continue;
    }
    bytes.push(byte);
  }

  String::from_utf8_lossy(&bytes).into_owned()
}

fn html_decode(value: &str) -> String {
  value
    .replace("&amp;", "&")
    .replace("&quot;", "\"")
    .replace("&apos;", "'")
    .replace("&#x27;", "'")
    .replace("&#39;", "'")
    .replace("&lt;", "<")
    .replace("&gt;", ">")
    .replace("&nbsp;", " ")
}

fn hex_value(byte: u8) -> Option<u8> {
  match byte {
    b'0'..=b'9' => Some(byte - b'0'),
    b'a'..=b'f' => Some(byte - b'a' + 10),
    b'A'..=b'F' => Some(byte - b'A' + 10),
    _ => None,
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn parses_duckduckgo_lite_results() {
    let html = r#"
      <a rel="nofollow" href="//duckduckgo.com/l/?uddg=https%3A%2F%2Fexample.com%2Fpith&amp;rut=abc" class='result-link'>Example <b>Pith</b></a>
      <td class='result-snippet'>A <b>local</b> search result &amp; snippet.</td>
    "#;

    let results = parse_duckduckgo_lite_results(html, 5);

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].title, "Example Pith");
    assert_eq!(results[0].url, "https://example.com/pith");
    assert_eq!(results[0].snippet, "A local search result & snippet.");
    assert_eq!(results[0].source, WEB_SEARCH_PROVIDER);
  }

  #[test]
  fn query_encoding_preserves_utf8() {
    assert_eq!(percent_encode("pith web search"), "pith+web+search");
    assert_eq!(percent_encode("LFM2.5/350M"), "LFM2.5%2F350M");
  }

  #[test]
  fn percent_decoding_handles_redirect_urls() {
    assert_eq!(
      percent_decode("https%3A%2F%2Fexample.com%2Fa%3Fb%3D1"),
      "https://example.com/a?b=1"
    );
  }
}
