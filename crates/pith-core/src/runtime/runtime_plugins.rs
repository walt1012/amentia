use std::path::{Path, PathBuf};

use pith_plugin_host::PluginCatalogEntry;

#[derive(Debug, Clone)]
pub(crate) struct RuntimePluginState {
  roots: Vec<PathBuf>,
  install_root: PathBuf,
  catalog: Vec<PluginCatalogEntry>,
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

  pub(crate) fn roots(&self) -> &[PathBuf] {
    &self.roots
  }

  pub(crate) fn install_root(&self) -> &Path {
    &self.install_root
  }

  pub(crate) fn catalog(&self) -> &[PluginCatalogEntry] {
    &self.catalog
  }

  pub(crate) fn catalog_len(&self) -> usize {
    self.catalog.len()
  }

  pub(crate) fn enabled_ready_count(&self) -> usize {
    self
      .catalog
      .iter()
      .filter(|plugin| plugin.enabled && plugin.status == "ready")
      .count()
  }

  pub(crate) fn contains_plugin_id(&self, plugin_id: &str) -> bool {
    self.catalog.iter().any(|plugin| plugin.id == plugin_id)
  }

  pub(crate) fn find(&self, plugin_id: &str) -> Option<&PluginCatalogEntry> {
    self.catalog.iter().find(|plugin| plugin.id == plugin_id)
  }

  pub(crate) fn replace_catalog(&mut self, catalog: Vec<PluginCatalogEntry>) {
    self.catalog = catalog;
  }

  pub(crate) fn configure_roots(&mut self, roots: Vec<PathBuf>, install_root: PathBuf) {
    self.roots = roots;
    self.install_root = install_root;
  }

  pub(crate) fn set_enabled(
    &mut self,
    plugin_id: &str,
    enabled: bool,
  ) -> Result<PluginCatalogEntry, PluginEnableError> {
    let Some(plugin) = self
      .catalog
      .iter_mut()
      .find(|plugin| plugin.id == plugin_id)
    else {
      return Err(PluginEnableError::NotFound);
    };
    if plugin.status != "ready" {
      return Err(PluginEnableError::InvalidManifest(
        plugin
          .validation_error
          .clone()
          .unwrap_or_else(|| "Plugin manifest is invalid".to_string()),
      ));
    }

    plugin.enabled = enabled;
    Ok(plugin.clone())
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PluginEnableError {
  NotFound,
  InvalidManifest(String),
}
