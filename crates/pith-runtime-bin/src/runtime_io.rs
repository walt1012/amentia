use std::io::{self, Write};
use std::sync::{Arc, Mutex};

use anyhow::Result;

#[derive(Clone)]
pub(crate) struct RuntimeOutput {
  stdout: Arc<Mutex<io::Stdout>>,
}

impl RuntimeOutput {
  pub(crate) fn stdout() -> Self {
    Self {
      stdout: Arc::new(Mutex::new(io::stdout())),
    }
  }

  pub(crate) fn write_json<T: serde::Serialize>(&self, payload: &T) -> Result<()> {
    let mut locked_stdout = self.stdout.lock().expect("stdout lock");
    writeln!(locked_stdout, "{}", serde_json::to_string(payload)?)?;
    locked_stdout.flush()?;
    Ok(())
  }
}
