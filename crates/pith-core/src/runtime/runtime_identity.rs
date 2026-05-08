#[derive(Debug, Clone)]
pub(crate) struct RuntimeIdentity {
  pub(crate) server_name: String,
  pub(crate) server_version: String,
}

impl RuntimeIdentity {
  pub(crate) fn pith_runtime() -> Self {
    Self {
      server_name: "pith-runtime".to_string(),
      server_version: env!("CARGO_PKG_VERSION").to_string(),
    }
  }
}
