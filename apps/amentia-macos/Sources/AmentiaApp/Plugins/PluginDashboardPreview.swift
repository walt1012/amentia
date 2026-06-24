enum PluginDashboardPreview {
  private static let previewLimit = 6

  static func catalogPreview(_ snapshot: PluginDashboardSnapshot) -> [PluginSummary] {
    snapshot.plugins
  }

  static func permissionPreview(_ snapshot: PluginDashboardSnapshot) -> [PluginSummary] {
    readyPluginList(snapshot)
  }

  static func invalidPlugins(_ snapshot: PluginDashboardSnapshot) -> [PluginSummary] {
    invalidPluginList(snapshot)
  }

  static func capabilityPreview(_ snapshot: PluginDashboardSnapshot) -> [PluginCapabilitySummary] {
    preview(snapshot.capabilities)
  }

  static func connectorPreview(_ snapshot: PluginDashboardSnapshot) -> [PluginConnectorSummary] {
    preview(snapshot.connectors)
  }

  static func commandPreview(_ snapshot: PluginDashboardSnapshot) -> [PluginCommandSummary] {
    preview(snapshot.commands)
  }

  static func commandConnectors(
    commandID: String,
    snapshot: PluginDashboardSnapshot
  ) -> [PluginConnectorSummary] {
    guard let command = snapshot.commands.first(where: { $0.id == commandID }) else {
      return []
    }

    return command.visibleConnectorIds.compactMap { connectorID in
      snapshot.connectors.first(where: { $0.id == connectorID })
    }
  }

  static func hookPreview(_ snapshot: PluginDashboardSnapshot) -> [PluginHookSummary] {
    preview(snapshot.hooks)
  }

  static func skillPreview(_ snapshot: PluginDashboardSnapshot) -> [PluginSkillSummary] {
    preview(snapshot.skills)
  }

  private static func preview<Value>(_ values: [Value]) -> [Value] {
    Array(values.prefix(previewLimit))
  }

  private static func readyPluginList(_ snapshot: PluginDashboardSnapshot) -> [PluginSummary] {
    snapshot.plugins.filter { $0.status == "ready" }
  }

  private static func invalidPluginList(_ snapshot: PluginDashboardSnapshot) -> [PluginSummary] {
    snapshot.plugins.filter { $0.status != "ready" }
  }
}
