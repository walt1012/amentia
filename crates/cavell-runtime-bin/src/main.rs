use std::io::{self, BufRead, Write};

use anyhow::Result;
use cavell_core::{handle_request, RuntimeContext};
use cavell_protocol::{JsonRpcRequest, JsonRpcResponse};

fn main() -> Result<()> {
  let context = RuntimeContext::new();
  let stdin = io::stdin();
  let mut stdout = io::stdout();

  for line in stdin.lock().lines() {
    let line = line?;
    let trimmed = line.trim();

    if trimmed.is_empty() {
      continue;
    }

    let response = match serde_json::from_str::<JsonRpcRequest>(trimmed) {
      Ok(request) => handle_request(&context, request),
      Err(error) => JsonRpcResponse::error(serde_json::Value::Null, -32700, error.to_string()),
    };

    writeln!(stdout, "{}", serde_json::to_string(&response)?)?;
    stdout.flush()?;
  }

  Ok(())
}
