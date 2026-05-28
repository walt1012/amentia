mod catalog;
mod io;
mod lifecycle;
mod manifest;
mod paths;
mod registry;
#[cfg(test)]
mod tests_bundled;
#[cfg(test)]
mod tests_lifecycle;
#[cfg(test)]
mod tests_manifest;
#[cfg(test)]
mod tests_registry;
#[cfg(test)]
mod tests_support;
mod types;
mod validation;

pub use catalog::{discover_plugins, discover_plugins_in_roots};
pub use lifecycle::{inspect_plugin_bundle, install_plugin_bundle, remove_local_plugin_bundle};
pub use manifest::{
  PluginAppConnectorManifest, PluginAuthPolicyManifest, PluginAuthor, PluginConnectorWorkflowManifest,
  PluginManifest, PluginMcpServerManifest, PluginSkillManifest,
};
pub use paths::{configured_plugin_install_root, configured_plugin_roots, default_plugin_root};
pub use registry::{
  build_capability_registry, build_command_registry, build_connector_registry, build_hook_registry,
};
pub use types::{
  PluginCapabilityRegistration, PluginCatalogEntry, PluginCommandEntry, PluginCommandEnvelopeEntry,
  PluginCommandEnvelopeFieldEntry, PluginCommandExecutionEntry, PluginConnectorEntry,
  PluginConnectorWorkflowEntry, PluginHookEntry, PluginRemovalRecord,
};
