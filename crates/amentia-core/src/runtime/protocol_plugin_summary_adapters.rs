use amentia_plugin_host::PluginCatalogEntry;
use amentia_protocol::PluginSummary as ProtocolPluginSummary;

pub(crate) fn to_protocol_plugin(plugin: PluginCatalogEntry) -> ProtocolPluginSummary {
  ProtocolPluginSummary {
    id: plugin.id,
    name: plugin.name,
    version: plugin.version,
    display_name: plugin.display_name,
    status: plugin.status,
    description: plugin.description,
    author_name: plugin.author_name,
    enabled: plugin.enabled,
    default_enabled: plugin.default_enabled,
    capabilities: plugin.capabilities,
    permissions: plugin.permissions,
    manifest_path: plugin.manifest_path,
    provenance: plugin.provenance,
    validation_error: plugin.validation_error,
    validation_hint: plugin.validation_hint,
  }
}
