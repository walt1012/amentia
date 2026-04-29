use pith_storage::RuntimeStore;

#[derive(Debug, Clone)]
pub(crate) struct RuntimePersistenceState {
  store: Option<RuntimeStore>,
}

impl RuntimePersistenceState {
  pub(crate) fn new(store: Option<RuntimeStore>) -> Self {
    Self { store }
  }

  pub(crate) fn persistent(store: RuntimeStore) -> Self {
    Self::new(Some(store))
  }

  pub(crate) fn in_memory() -> Self {
    Self::new(None)
  }

  pub(crate) fn store(&self) -> Option<&RuntimeStore> {
    self.store.as_ref()
  }

  #[cfg(test)]
  pub(crate) fn set_store_for_testing(&mut self, store: RuntimeStore) {
    self.store = Some(store);
  }
}
