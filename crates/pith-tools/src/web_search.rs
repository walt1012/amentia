use std::env;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::time::Duration;

use anyhow::{bail, Context, Result};
use pith_process::{
  configure_process_group, join_bounded_pipe_reader, read_bounded_pipe_in_background,
  wait_for_child, ChildExitReason,
};

use crate::types::WebSearchResult;
use crate::web_search_parser::parse_duckduckgo_lite_results;

const WEB_SEARCH_PROVIDER: &str = "DuckDuckGo Lite";
const WEB_SEARCH_ENDPOINT: &str = "https://lite.duckduckgo.com/lite/";
const WEB_SEARCH_CLIENT: &str = "curl";
const WEB_SEARCH_FIXTURE_CLIENT: &str = "fixture";
const WEB_SEARCH_FIXTURE_PATH_ENV: &str = "PITH_WEB_SEARCH_FIXTURE_PATH";
const WEB_SEARCH_CURL_TIMEOUT_SECONDS: &str = "15";
const WEB_SEARCH_CONNECT_TIMEOUT_SECONDS: &str = "8";
const WEB_SEARCH_MAX_BYTES: &str = "1048576";
const WEB_SEARCH_OUTPUT_LIMIT: usize = 1_048_576;
const WEB_SEARCH_PROCESS_TIMEOUT: Duration = Duration::from_secs(20);
const WEB_SEARCH_POLL_INTERVAL: Duration = Duration::from_millis(50);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WebSearchStatus {
  pub provider: String,
  pub client: String,
  pub available: bool,
  pub detail: String,
}

pub fn web_search(query: &str, max_results: usize) -> Result<Vec<WebSearchResult>> {
  web_search_with_cancellation(query, max_results, || false)
}

pub fn web_search_timeout_seconds() -> u64 {
  WEB_SEARCH_PROCESS_TIMEOUT.as_secs()
}

pub fn web_search_status() -> WebSearchStatus {
  if let Some(fixture_path) = configured_web_search_fixture_path() {
    let available = fixture_path.is_file();
    return WebSearchStatus {
      provider: WEB_SEARCH_PROVIDER.to_string(),
      client: WEB_SEARCH_FIXTURE_CLIENT.to_string(),
      available,
      detail: if available {
        format!(
          "Built-in web search uses a deterministic fixture at {}.",
          fixture_path.display()
        )
      } else {
        format!(
          "Built-in web search fixture was configured, but {} was not found.",
          fixture_path.display()
        )
      },
    };
  }

  let command_path = curl_command_path();
  let available = command_path.is_some();
  WebSearchStatus {
    provider: WEB_SEARCH_PROVIDER.to_string(),
    client: WEB_SEARCH_CLIENT.to_string(),
    available,
    detail: if let Some(command_path) = command_path {
      format!(
        "Built-in web search uses {} through {} with a {} second process timeout.",
        WEB_SEARCH_PROVIDER,
        command_path,
        WEB_SEARCH_PROCESS_TIMEOUT.as_secs()
      )
    } else {
      "Built-in web search is enabled, but curl was not found on PATH or standard system paths."
        .to_string()
    },
  }
}

pub fn web_search_with_cancellation<F>(
  query: &str,
  max_results: usize,
  is_cancelled: F,
) -> Result<Vec<WebSearchResult>>
where
  F: Fn() -> bool,
{
  let trimmed_query = query.trim();
  if trimmed_query.is_empty() {
    bail!("web search query must not be empty");
  }
  if max_results == 0 {
    return Ok(vec![]);
  }

  if let Some(fixture_path) = configured_web_search_fixture_path() {
    return web_search_from_fixture(&fixture_path, max_results, is_cancelled);
  }

  let url = format!(
    "{}?q={}",
    WEB_SEARCH_ENDPOINT,
    percent_encode(trimmed_query)
  );
  let output = run_web_search_request(&url, is_cancelled)
    .with_context(|| "failed to execute bounded web search request")?;
  if output.cancelled {
    bail!("web search cancelled");
  }
  if output.timed_out {
    bail!(
      "web search timed out after {} seconds",
      WEB_SEARCH_PROCESS_TIMEOUT.as_secs()
    );
  }
  if !output.status.success() {
    let stderr = String::from_utf8_lossy(&output.stderr);
    bail!("web search request failed: {}", stderr.trim());
  }

  let html = String::from_utf8_lossy(&output.stdout);
  Ok(parse_duckduckgo_lite_results(
    &html,
    max_results,
    WEB_SEARCH_PROVIDER,
  ))
}

fn web_search_from_fixture<F>(
  fixture_path: &Path,
  max_results: usize,
  is_cancelled: F,
) -> Result<Vec<WebSearchResult>>
where
  F: Fn() -> bool,
{
  if is_cancelled() {
    bail!("web search cancelled");
  }
  let html = std::fs::read_to_string(fixture_path).with_context(|| {
    format!(
      "failed to read web search fixture at {}",
      fixture_path.display()
    )
  })?;
  if is_cancelled() {
    bail!("web search cancelled");
  }

  Ok(parse_duckduckgo_lite_results(
    &html,
    max_results,
    WEB_SEARCH_PROVIDER,
  ))
}

struct WebSearchHttpOutput {
  status: ExitStatus,
  stdout: Vec<u8>,
  stderr: Vec<u8>,
  timed_out: bool,
  cancelled: bool,
}

fn run_web_search_request<F>(url: &str, is_cancelled: F) -> Result<WebSearchHttpOutput>
where
  F: Fn() -> bool,
{
  let mut child = build_curl_command(url)
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn()
    .with_context(|| "failed to start curl for web search")?;
  let stdout_reader = child
    .stdout
    .take()
    .map(|reader| read_bounded_pipe_in_background(reader, WEB_SEARCH_OUTPUT_LIMIT));
  let stderr_reader = child
    .stderr
    .take()
    .map(|reader| read_bounded_pipe_in_background(reader, WEB_SEARCH_OUTPUT_LIMIT));
  let wait = wait_for_child(
    &mut child,
    WEB_SEARCH_PROCESS_TIMEOUT,
    WEB_SEARCH_POLL_INTERVAL,
    Duration::from_millis(200),
    is_cancelled,
  )?;

  Ok(WebSearchHttpOutput {
    status: wait.status,
    stdout: join_bounded_pipe_reader(stdout_reader).bytes,
    stderr: join_bounded_pipe_reader(stderr_reader).bytes,
    timed_out: wait.reason == ChildExitReason::TimedOut,
    cancelled: wait.reason == ChildExitReason::Cancelled,
  })
}

fn build_curl_command(url: &str) -> Command {
  let command_path = curl_command_path().unwrap_or_else(|| WEB_SEARCH_CLIENT.to_string());
  let mut process = Command::new(command_path);
  process
    .args([
      "--silent",
      "--show-error",
      "--location",
      "--max-time",
      WEB_SEARCH_CURL_TIMEOUT_SECONDS,
      "--connect-timeout",
      WEB_SEARCH_CONNECT_TIMEOUT_SECONDS,
      "--max-filesize",
      WEB_SEARCH_MAX_BYTES,
      "--user-agent",
      "Pith/0.1",
      "--url",
      url,
    ])
    .stdin(Stdio::null());
  configure_process_group(&mut process);
  process
}

fn curl_command_path() -> Option<String> {
  if command_available(WEB_SEARCH_CLIENT) {
    return Some(WEB_SEARCH_CLIENT.to_string());
  }

  fallback_curl_path().map(str::to_string)
}

fn configured_web_search_fixture_path() -> Option<PathBuf> {
  env::var_os(WEB_SEARCH_FIXTURE_PATH_ENV).map(PathBuf::from)
}

#[cfg(target_os = "macos")]
fn fallback_curl_path() -> Option<&'static str> {
  let path = "/usr/bin/curl";
  Path::new(path).is_file().then_some(path)
}

#[cfg(not(target_os = "macos"))]
fn fallback_curl_path() -> Option<&'static str> {
  None
}

fn command_available(command: &str) -> bool {
  let command_path = Path::new(command);
  if command_path.components().count() > 1 {
    return command_path.is_file();
  }

  env::var_os("PATH")
    .map(|paths| {
      env::split_paths(&paths).any(|directory| {
        command_candidates(&directory, command)
          .into_iter()
          .any(|candidate| candidate.is_file())
      })
    })
    .unwrap_or(false)
}

#[cfg(windows)]
fn command_candidates(directory: &Path, command: &str) -> Vec<PathBuf> {
  if Path::new(command).extension().is_some() {
    return vec![directory.join(command)];
  }

  let mut candidates = vec![directory.join(command)];
  for extension in env::var("PATHEXT")
    .unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".to_string())
    .split(';')
    .filter(|extension| !extension.is_empty())
  {
    candidates.push(directory.join(format!("{command}{extension}")));
  }
  candidates
}

#[cfg(not(windows))]
fn command_candidates(directory: &Path, command: &str) -> Vec<PathBuf> {
  vec![directory.join(command)]
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

#[cfg(test)]
mod tests {
  use super::*;
  use std::ffi::OsString;
  use std::sync::Mutex;

  static WEB_SEARCH_ENV_LOCK: Mutex<()> = Mutex::new(());

  #[test]
  fn query_encoding_preserves_utf8() {
    assert_eq!(percent_encode("pith web search"), "pith+web+search");
    assert_eq!(percent_encode("LFM2.5/350M"), "LFM2.5%2F350M");
  }

  #[test]
  fn web_search_pipe_reader_bounds_retained_output() {
    let input = std::io::Cursor::new(vec![b'x'; WEB_SEARCH_OUTPUT_LIMIT + 512]);
    let output = join_bounded_pipe_reader(Some(read_bounded_pipe_in_background(
      input,
      WEB_SEARCH_OUTPUT_LIMIT,
    )));

    assert_eq!(output.bytes.len(), WEB_SEARCH_OUTPUT_LIMIT);
    assert_eq!(output.source_byte_count, WEB_SEARCH_OUTPUT_LIMIT + 512);
  }

  #[test]
  fn web_search_status_reports_provider_and_client() {
    let _guard = WEB_SEARCH_ENV_LOCK.lock().expect("web search env lock");
    let _env_guard = EnvVarGuard::remove(WEB_SEARCH_FIXTURE_PATH_ENV);
    let status = web_search_status();

    assert_eq!(status.provider, WEB_SEARCH_PROVIDER);
    assert_eq!(status.client, WEB_SEARCH_CLIENT);
    assert!(!status.detail.is_empty());
  }

  #[test]
  fn web_search_fixture_runs_without_network() {
    let _guard = WEB_SEARCH_ENV_LOCK.lock().expect("web search env lock");
    let fixture_path = env::temp_dir().join(format!(
      "pith-web-search-fixture-{}.html",
      std::process::id()
    ));
    std::fs::write(
      &fixture_path,
      r#"
        <a rel="nofollow" href="//duckduckgo.com/l/?uddg=https%3A%2F%2Fexample.com%2Fpith&amp;rut=abc" class='result-link'>Pith fixture result</a>
        <td class='result-snippet'>Deterministic local web search result.</td>
      "#,
    )
    .expect("write web search fixture");
    let _env_guard = EnvVarGuard::set(WEB_SEARCH_FIXTURE_PATH_ENV, fixture_path.as_os_str());

    let status = web_search_status();
    let results =
      web_search_with_cancellation("Pith local model", 5, || false).expect("fixture web search");

    let _ = std::fs::remove_file(&fixture_path);
    assert_eq!(status.client, WEB_SEARCH_FIXTURE_CLIENT);
    assert!(status.available);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].title, "Pith fixture result");
    assert_eq!(results[0].url, "https://example.com/pith");
  }

  struct EnvVarGuard {
    key: &'static str,
    previous: Option<OsString>,
  }

  impl EnvVarGuard {
    fn set(key: &'static str, value: &std::ffi::OsStr) -> Self {
      let previous = env::var_os(key);
      env::set_var(key, value);
      Self { key, previous }
    }

    fn remove(key: &'static str) -> Self {
      let previous = env::var_os(key);
      env::remove_var(key);
      Self { key, previous }
    }
  }

  impl Drop for EnvVarGuard {
    fn drop(&mut self) {
      if let Some(previous) = self.previous.as_ref() {
        env::set_var(self.key, previous);
      } else {
        env::remove_var(self.key);
      }
    }
  }
}
