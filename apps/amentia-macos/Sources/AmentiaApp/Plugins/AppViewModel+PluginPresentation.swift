import Foundation

@MainActor
extension AppViewModel {
  func pluginCountSummary() -> String {
    PluginDashboardPresenter.pluginCountSummary(pluginDashboardSnapshot)
  }

  func localPluginCountSummary() -> String {
    PluginDashboardPresenter.localPluginCountSummary(pluginDashboardSnapshot)
  }

  func pluginDetailSummary() -> String {
    PluginDashboardPresenter.pluginDetailSummary(pluginDashboardSnapshot)
  }

  func pluginSurfaceSummary() -> String {
    let specificSummaries = [
      pluginConnectorCountSummary(),
      pluginCommandCountSummary(),
      pluginSkillCountSummary(),
      pluginHookCountSummary(),
    ].filter { summary in
      !summary.hasPrefix("No ") && !summary.contains("not loaded")
    }

    if !specificSummaries.isEmpty {
      return specificSummaries.joined(separator: " | ")
    }

    let capabilitySummary = pluginRegistryCountSummary()
    if !capabilitySummary.hasPrefix("No ") && !capabilitySummary.contains("not loaded") {
      return capabilitySummary
    }

    return "Install plugins to add actions, connections, skills, and checks."
  }

  func pluginCatalogPreview() -> [PluginSummary] {
    PluginDashboardPresenter.catalogPreview(pluginDashboardSnapshot)
  }

  func pluginPermissionCountSummary() -> String {
    PluginDashboardPresenter.permissionCountSummary(pluginDashboardSnapshot)
  }

  func pluginPermissionDetailSummary() -> String {
    PluginDashboardPresenter.permissionDetailSummary(pluginDashboardSnapshot)
  }

  func pluginPermissionPreview() -> [PluginSummary] {
    PluginDashboardPresenter.permissionPreview(pluginDashboardSnapshot)
  }

  func invalidPluginCountSummary() -> String {
    PluginDashboardPresenter.invalidPluginCountSummary(pluginDashboardSnapshot)
  }

  func invalidPluginDetailSummary() -> String {
    PluginDashboardPresenter.invalidPluginDetailSummary(pluginDashboardSnapshot)
  }

  func invalidPlugins() -> [PluginSummary] {
    PluginDashboardPresenter.invalidPlugins(pluginDashboardSnapshot)
  }

  func pluginRegistryCountSummary() -> String {
    PluginDashboardPresenter.registryCountSummary(pluginDashboardSnapshot)
  }

  func pluginRegistryDetailSummary() -> String {
    PluginDashboardPresenter.registryDetailSummary(pluginDashboardSnapshot)
  }

  func pluginCapabilityPreview() -> [PluginCapabilitySummary] {
    PluginDashboardPresenter.capabilityPreview(pluginDashboardSnapshot)
  }

  func pluginConnectorCountSummary() -> String {
    PluginDashboardPresenter.connectorCountSummary(pluginDashboardSnapshot)
  }

  func pluginConnectorDetailSummary() -> String {
    PluginDashboardPresenter.connectorDetailSummary(pluginDashboardSnapshot)
  }

  func pluginConnectorPreview() -> [PluginConnectorSummary] {
    PluginDashboardPresenter.connectorPreview(pluginDashboardSnapshot)
  }

  func pluginCommandCountSummary() -> String {
    PluginDashboardPresenter.commandCountSummary(pluginDashboardSnapshot)
  }

  func pluginCommandDetailSummary() -> String {
    PluginDashboardPresenter.commandDetailSummary(pluginDashboardSnapshot)
  }

  func pluginCommandPreview() -> [PluginCommandSummary] {
    PluginDashboardPresenter.commandPreview(pluginDashboardSnapshot)
  }

  func pluginCommandConnectors(commandID: String) -> [PluginConnectorSummary] {
    PluginDashboardPresenter.commandConnectors(
      commandID: commandID,
      snapshot: pluginDashboardSnapshot
    )
  }

  func pluginHookCountSummary() -> String {
    PluginDashboardPresenter.hookCountSummary(pluginDashboardSnapshot)
  }

  func pluginHookDetailSummary() -> String {
    PluginDashboardPresenter.hookDetailSummary(pluginDashboardSnapshot)
  }

  func pluginHookPreview() -> [PluginHookSummary] {
    PluginDashboardPresenter.hookPreview(pluginDashboardSnapshot)
  }

  func pluginSkillCountSummary() -> String {
    PluginDashboardPresenter.skillCountSummary(pluginDashboardSnapshot)
  }

  func pluginSkillDetailSummary() -> String {
    PluginDashboardPresenter.skillDetailSummary(pluginDashboardSnapshot)
  }

  func pluginSkillPreview() -> [PluginSkillSummary] {
    PluginDashboardPresenter.skillPreview(pluginDashboardSnapshot)
  }

  func canDisablePluginGuidance(skill: PluginSkillSummary) -> Bool {
    pluginSummary(pluginID: skill.pluginID)?.enabled == true
      && canSetPluginEnabled(pluginID: skill.pluginID)
  }

  func canDisablePluginCheck(hook: PluginHookSummary) -> Bool {
    pluginSummary(pluginID: hook.pluginID)?.enabled == true
      && canSetPluginEnabled(pluginID: hook.pluginID)
  }
}
