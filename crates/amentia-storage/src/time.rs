use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};

pub(crate) fn current_timestamp() -> Result<i64> {
  Ok(
    SystemTime::now()
      .duration_since(UNIX_EPOCH)
      .context("system time is earlier than the unix epoch")?
      .as_secs() as i64,
  )
}
