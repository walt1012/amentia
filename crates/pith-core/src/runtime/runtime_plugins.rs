use std::path::PathBuf;

use pith_plugin_host::PluginCatalogEntry;

#[derive(Debug, Clone)]
pub(crate) struct RuntimePluginState {
  pub(crate) roots: Vec<PathBuf>,
  pub(crate) install_root: PathBuf,
  pub(crate) catalog: Vec<PluginCatalogEntry>,
}

impl RuntimePluginState {
  pub(crate) fn new(
    roots: Vec<PathBuf>,
    install_root: PathBuf,
    catalog: Vec<PluginCatalogEntry>,
  ) -> Self {
    Self {
      roots,
      install_root,
      catalog,
    }
  }
}
