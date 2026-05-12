use anyhow::Result;
use pith_plugin_host::{configured_plugin_install_root, configured_plugin_roots};

use crate::plugin_catalog_state::{apply_plugin_states, load_plugin_catalog};
use crate::runtime_context::RuntimeContext;
use crate::runtime_execution::RuntimeExecutionState;
use crate::runtime_identity::RuntimeIdentity;
use crate::runtime_memory::RuntimeMemoryState;
use crate::runtime_model::RuntimeModelState;
use crate::runtime_persistence::RuntimePersistenceState;
use crate::runtime_plugins::RuntimePluginState;
use crate::runtime_sequences::RuntimeSequenceState;
use crate::runtime_threads::RuntimeThreadState;
use crate::runtime_workspace::RuntimeWorkspaceState;

impl RuntimeContext {
  pub fn new() -> Result<Self> {
    let bootstrap = RuntimePersistenceState::load_default_bootstrap()?;
    let plugin_roots = configured_plugin_roots();
    let plugin_install_root = configured_plugin_install_root();
    let plugins = apply_plugin_states(
      load_plugin_catalog(&plugin_roots)?,
      &bootstrap.plugin_states,
    );

    Ok(Self {
      identity: RuntimeIdentity::pith_runtime(),
      model_state: RuntimeModelState::new_default(true),
      persistence_state: bootstrap.persistence_state,
      memory_state: bootstrap.memory_state,
      thread_state: bootstrap.thread_state,
      workspace_state: bootstrap.workspace_state,
      plugin_state: RuntimePluginState::new(
        plugin_roots,
        plugin_install_root,
        plugins,
        bootstrap.plugin_connector_credentials,
      ),
      execution_state: bootstrap.execution_state,
      sequence_state: bootstrap.sequence_state,
    })
  }

  pub fn new_in_memory() -> Self {
    let plugin_roots = configured_plugin_roots();
    let plugin_install_root = configured_plugin_install_root();
    Self {
      identity: RuntimeIdentity::pith_runtime(),
      model_state: RuntimeModelState::new_default(false),
      persistence_state: RuntimePersistenceState::in_memory(),
      memory_state: RuntimeMemoryState::new(1, vec![]),
      thread_state: RuntimeThreadState::empty(),
      workspace_state: RuntimeWorkspaceState::new(None),
      plugin_state: RuntimePluginState::new(
        plugin_roots.clone(),
        plugin_install_root,
        load_plugin_catalog(&plugin_roots).unwrap_or_default(),
        Default::default(),
      ),
      execution_state: RuntimeExecutionState::empty(),
      sequence_state: RuntimeSequenceState::new(1, 1),
    }
  }

  pub fn cancel_running_work(&mut self) {
    self.execution_state.cancel_running_work();
  }

  pub fn recover_after_request_panic(&mut self) -> Result<()> {
    self.execution_state.clear_running_work_after_recovery();
    self.persist_runtime_state()
  }
}

impl Default for RuntimeContext {
  fn default() -> Self {
    Self::new_in_memory()
  }
}
