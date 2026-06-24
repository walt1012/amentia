enum PluginDashboardPresenter {
  static func pluginCountSummary(_ snapshot: PluginDashboardSnapshot) -> String {
    if snapshot.plugins.isEmpty {
      return "No plugins yet."
    }

    let readyCount = readyPluginList(snapshot).count
    let invalidCount = snapshot.plugins.count - readyCount
    if invalidCount == 0 {
      return "\(plural(readyCount, "plugin")) ready"
    }

    return "\(readyCount) ready, \(invalidCount) need attention"
  }

  static func localPluginCountSummary(_ snapshot: PluginDashboardSnapshot) -> String {
    let localPlugins = snapshot.plugins.filter { $0.provenance == "local" }

    if localPlugins.isEmpty {
      return "No local plugins installed yet."
    }

    return "\(localPlugins.count) local \(pluralName(localPlugins.count, "plugin"))"
  }

  static func permissionCountSummary(_ snapshot: PluginDashboardSnapshot) -> String {
    let readyPlugins = readyPluginList(snapshot)
    let uniquePermissions = Set(readyPlugins.flatMap(\.permissions))

    guard !readyPlugins.isEmpty else {
      return "Plugin permissions are not loaded yet."
    }

    if uniquePermissions.isEmpty {
      let pluginLabel = pluralName(readyPlugins.count, "plugin")
      return "\(readyPlugins.count) ready \(pluginLabel), no extra permissions"
    }

    let permissionSummary = plural(uniquePermissions.count, "permission")
    let pluginLabel = pluralName(readyPlugins.count, "plugin")
    return "\(permissionSummary) across \(readyPlugins.count) ready \(pluginLabel)"
  }

  static func invalidPluginCountSummary(_ snapshot: PluginDashboardSnapshot) -> String {
    let invalidPlugins = invalidPluginList(snapshot)

    if invalidPlugins.isEmpty {
      return "No Setup Issues"
    }

    return "\(invalidPlugins.count) \(pluralName(invalidPlugins.count, "plugin setup issue"))"
  }

  static func registryCountSummary(_ snapshot: PluginDashboardSnapshot) -> String {
    guard let registrySummary = snapshot.registrySummary else {
      return "Plugin capabilities are not loaded yet."
    }

    if registrySummary.totalCapabilityCount == 0 {
      return "No plugin capabilities yet"
    }

    let capabilitySummary = plural(
      registrySummary.totalCapabilityCount,
      "capability",
      plural: "capabilities"
    )
    let pluginLabel = pluralName(registrySummary.enabledPluginCount, "plugin")
    return "\(capabilitySummary) from \(registrySummary.enabledPluginCount) enabled \(pluginLabel)"
  }

  static func connectorCountSummary(_ snapshot: PluginDashboardSnapshot) -> String {
    if snapshot.connectors.isEmpty {
      return "No connections yet"
    }

    let readyCount = snapshot.connectors.filter { $0.status == "ready" }.count
    let needsAuthCount = snapshot.connectors.filter { connector in
      connectorAuthorizationStatus(connector) == "needs sign in"
    }.count
    let authorizedCount = snapshot.connectors.filter { connector in
      connectorAuthorizationStatus(connector) == "authorized locally"
    }.count
    return statusSummary(
      total: plural(snapshot.connectors.count, "connection"),
      nonZeroParts: [
        (readyCount, "ready"),
        (needsAuthCount, "need sign in"),
        (authorizedCount, "authorized")
      ]
    )
  }

  static func commandCountSummary(_ snapshot: PluginDashboardSnapshot) -> String {
    if snapshot.commands.isEmpty {
      return "No actions yet"
    }

    let readyCount = snapshot.commands.filter { $0.runStatus == "ready" }.count
    let blockedCount = snapshot.commands.count - readyCount
    let approvalCount = snapshot.commands.filter { $0.approvalRequired }.count
    return statusSummary(
      total: plural(snapshot.commands.count, "action"),
      nonZeroParts: [
        (readyCount, "ready"),
        (blockedCount, "blocked"),
        (approvalCount, "approval gated")
      ]
    )
  }

  static func hookCountSummary(_ snapshot: PluginDashboardSnapshot) -> String {
    if snapshot.hooks.isEmpty {
      return "No checks yet"
    }

    return plural(snapshot.hooks.count, "check")
  }

  static func skillCountSummary(_ snapshot: PluginDashboardSnapshot) -> String {
    if snapshot.skills.isEmpty {
      return "No skills yet"
    }

    let readyCount = snapshot.skills.filter { $0.status == "ready" }.count
    let blockedCount = snapshot.skills.count - readyCount
    if blockedCount == 0 {
      return "\(plural(readyCount, "skill")) ready"
    }
    return "\(plural(snapshot.skills.count, "skill")) | \(blockedCount) need review"
  }

  private static func plural(
    _ count: Int,
    _ singular: String,
    plural pluralName: String? = nil
  ) -> String {
    "\(count) \(self.pluralName(count, singular, plural: pluralName))"
  }

  private static func pluralName(
    _ count: Int,
    _ singular: String,
    plural pluralName: String? = nil
  ) -> String {
    count == 1 ? singular : (pluralName ?? "\(singular)s")
  }

  private static func statusSummary(
    total: String,
    nonZeroParts: [(count: Int, label: String)]
  ) -> String {
    ([total] + nonZeroParts.compactMap { part in
      part.count > 0 ? "\(part.count) \(part.label)" : nil
    })
    .joined(separator: " | ")
  }

  private static func readyPluginList(_ snapshot: PluginDashboardSnapshot) -> [PluginSummary] {
    snapshot.plugins.filter { $0.status == "ready" }
  }

  private static func invalidPluginList(_ snapshot: PluginDashboardSnapshot) -> [PluginSummary] {
    snapshot.plugins.filter { $0.status != "ready" }
  }

  private static func connectorAuthorizationStatus(
    _ connector: PluginConnectorSummary
  ) -> String {
    PluginStatusDisplay.authorizationStatus(
      connector.authStatus,
      authRequired: connector.authRequired,
      credentialPresent: connector.credentialPresent,
      credentialSecretPresent: connector.credentialSecretPresent
    )
  }
}
