use std::collections::HashMap;
use std::path::{Path, PathBuf};

use pith_plugin_host::{build_connector_registry, PluginCatalogEntry, PluginConnectorEntry};
use pith_storage::StoredPluginConnectorCredential;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PluginConnectorCredentialState {
  pub(crate) connector_id: String,
  pub(crate) plugin_id: String,
  pub(crate) credential_store: String,
  pub(crate) credential_label: String,
  pub(crate) authorized_at: i64,
  pub(crate) updated_at: i64,
}

impl From<StoredPluginConnectorCredential> for PluginConnectorCredentialState {
  fn from(record: StoredPluginConnectorCredential) -> Self {
    Self {
      connector_id: record.connector_id,
      plugin_id: record.plugin_id,
      credential_store: record.credential_store,
      credential_label: record.credential_label,
      authorized_at: record.authorized_at,
      updated_at: record.updated_at,
    }
  }
}

impl From<&PluginConnectorCredentialState> for StoredPluginConnectorCredential {
  fn from(state: &PluginConnectorCredentialState) -> Self {
    Self {
      connector_id: state.connector_id.clone(),
      plugin_id: state.plugin_id.clone(),
      credential_store: state.credential_store.clone(),
      credential_label: state.credential_label.clone(),
      authorized_at: state.authorized_at,
      updated_at: state.updated_at,
    }
  }
}

#[derive(Debug, Clone)]
pub(crate) struct RuntimePluginState {
  roots: Vec<PathBuf>,
  install_root: PathBuf,
  catalog: Vec<PluginCatalogEntry>,
  connector_credentials: HashMap<String, PluginConnectorCredentialState>,
}

impl RuntimePluginState {
  pub(crate) fn new(
    roots: Vec<PathBuf>,
    install_root: PathBuf,
    catalog: Vec<PluginCatalogEntry>,
    connector_credentials: HashMap<String, PluginConnectorCredentialState>,
  ) -> Self {
    Self {
      roots,
      install_root,
      catalog,
      connector_credentials,
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

  pub(crate) fn snapshot_catalog(&self) -> Vec<PluginCatalogEntry> {
    self.catalog.clone()
  }

  pub(crate) fn catalog_len(&self) -> usize {
    self.catalog.len()
  }

  pub(crate) fn connector_entries(&self) -> Vec<PluginConnectorEntry> {
    build_connector_registry(&self.catalog)
  }

  pub(crate) fn connector_credential(
    &self,
    connector_id: &str,
  ) -> Option<&PluginConnectorCredentialState> {
    self.connector_credentials.get(connector_id)
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

  pub(crate) fn set_connector_credential(
    &mut self,
    credential: PluginConnectorCredentialState,
  ) {
    self
      .connector_credentials
      .insert(credential.connector_id.clone(), credential);
  }

  pub(crate) fn clear_connector_credential(
    &mut self,
    connector_id: &str,
  ) -> Option<PluginConnectorCredentialState> {
    self.connector_credentials.remove(connector_id)
  }

  pub(crate) fn clear_connector_credentials_for_plugin(&mut self, plugin_id: &str) {
    self
      .connector_credentials
      .retain(|_, credential| credential.plugin_id != plugin_id);
  }

  #[cfg(test)]
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
