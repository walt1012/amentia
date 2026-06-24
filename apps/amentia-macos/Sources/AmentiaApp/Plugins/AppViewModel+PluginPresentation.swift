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
    PluginDashboardDetailPresenter.pluginDetailSummary(pluginDashboardSnapshot)
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
    PluginDashboardPreview.catalogPreview(pluginDashboardSnapshot)
  }

  func pluginPermissionCountSummary() -> String {
    PluginDashboardPresenter.permissionCountSummary(pluginDashboardSnapshot)
  }

  func pluginPermissionDetailSummary() -> String {
    PluginDashboardDetailPresenter.permissionDetailSummary(pluginDashboardSnapshot)
  }

  func pluginPermissionPreview() -> [PluginSummary] {
    PluginDashboardPreview.permissionPreview(pluginDashboardSnapshot)
  }

  func invalidPluginCountSummary() -> String {
    PluginDashboardPresenter.invalidPluginCountSummary(pluginDashboardSnapshot)
  }

  func invalidPluginDetailSummary() -> String {
    PluginDashboardDetailPresenter.invalidPluginDetailSummary(pluginDashboardSnapshot)
  }

  func invalidPlugins() -> [PluginSummary] {
    PluginDashboardPreview.invalidPlugins(pluginDashboardSnapshot)
  }

  func pluginRegistryCountSummary() -> String {
    PluginDashboardPresenter.registryCountSummary(pluginDashboardSnapshot)
  }

  func pluginRegistryDetailSummary() -> String {
    PluginDashboardDetailPresenter.registryDetailSummary(pluginDashboardSnapshot)
  }

  func pluginCapabilityPreview() -> [PluginCapabilitySummary] {
    PluginDashboardPreview.capabilityPreview(pluginDashboardSnapshot)
  }

  func pluginConnectorCountSummary() -> String {
    PluginDashboardPresenter.connectorCountSummary(pluginDashboardSnapshot)
  }

  func pluginConnectorDetailSummary() -> String {
    PluginDashboardDetailPresenter.connectorDetailSummary(pluginDashboardSnapshot)
  }

  func pluginConnectorPreview() -> [PluginConnectorSummary] {
    PluginDashboardPreview.connectorPreview(pluginDashboardSnapshot)
  }

  func pluginCommandCountSummary() -> String {
    PluginDashboardPresenter.commandCountSummary(pluginDashboardSnapshot)
  }

  func pluginCommandDetailSummary() -> String {
    PluginDashboardDetailPresenter.commandDetailSummary(pluginDashboardSnapshot)
  }

  func pluginCommandPreview() -> [PluginCommandSummary] {
    PluginDashboardPreview.commandPreview(pluginDashboardSnapshot)
  }

  func pluginCommandConnectors(commandID: String) -> [PluginConnectorSummary] {
    PluginDashboardPreview.commandConnectors(
      commandID: commandID,
      snapshot: pluginDashboardSnapshot
    )
  }

  func pluginHookCountSummary() -> String {
    PluginDashboardPresenter.hookCountSummary(pluginDashboardSnapshot)
  }

  func pluginHookDetailSummary() -> String {
    PluginDashboardDetailPresenter.hookDetailSummary(pluginDashboardSnapshot)
  }

  func pluginHookPreview() -> [PluginHookSummary] {
    PluginDashboardPreview.hookPreview(pluginDashboardSnapshot)
  }

  func pluginSkillCountSummary() -> String {
    PluginDashboardPresenter.skillCountSummary(pluginDashboardSnapshot)
  }

  func pluginSkillDetailSummary() -> String {
    PluginDashboardDetailPresenter.skillDetailSummary(pluginDashboardSnapshot)
  }

  func pluginSkillPreview() -> [PluginSkillSummary] {
    PluginDashboardPreview.skillPreview(pluginDashboardSnapshot)
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
