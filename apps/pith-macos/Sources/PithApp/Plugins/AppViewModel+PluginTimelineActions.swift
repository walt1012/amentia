import Foundation

@MainActor
extension AppViewModel {
  func canAuthorizePluginConnector(from entry: TimelineEntry) -> Bool {
    guard isPluginConnectorAuthorizationEntry(entry),
          let connectorID = pluginConnectorID(from: entry)
    else {
      return false
    }

    return canAuthorizePluginConnector(connectorID: connectorID)
  }

  func canEnablePlugin(from entry: TimelineEntry) -> Bool {
    guard isPluginRecoveryEntry(entry),
          let pluginID = pluginID(from: entry),
          let plugin = pluginSummary(pluginID: pluginID),
          !plugin.enabled
    else {
      return false
    }

    return canSetPluginEnabled(pluginID: pluginID)
  }

  func enablePlugin(from entry: TimelineEntry) {
    guard canEnablePlugin(from: entry),
          let pluginID = pluginID(from: entry)
    else {
      runtimeDetail = "Plugin enable action is unavailable."
      return
    }

    setPluginEnabled(pluginID: pluginID, enabled: true)
  }

  func authorizePluginConnector(from entry: TimelineEntry) {
    guard canAuthorizePluginConnector(from: entry),
          let connectorID = pluginConnectorID(from: entry)
    else {
      runtimeDetail = "Plugin connector authorization is unavailable."
      return
    }

    authorizePluginConnector(connectorID: connectorID)
  }

  func canRevealPluginSource(from entry: TimelineEntry) -> Bool {
    isPluginSourceRevealEntry(entry)
      && pluginSourcePath(from: entry) != nil
  }

  func canRefreshPlugins(from entry: TimelineEntry) -> Bool {
    isPluginSourceRefreshEntry(entry)
      && pluginSourcePath(from: entry) != nil
      && canRefreshPlugins()
  }

  func revealPluginSource(from entry: TimelineEntry) {
    guard canRevealPluginSource(from: entry),
          let sourcePath = pluginSourcePath(from: entry)
    else {
      runtimeDetail = "Plugin source path is unavailable."
      return
    }

    runtimeDetail = FileRevealService.revealFilePath(
      sourcePath,
      successDetail: "Revealed plugin source."
    )
  }

  func refreshPlugins(from entry: TimelineEntry) async {
    guard canRefreshPlugins(from: entry) else {
      runtimeDetail = pluginRefreshDisabledReason() ?? "Plugin refresh is unavailable."
      return
    }

    await refreshPlugins()
  }

  func revealPluginManifest(pluginID: String) {
    guard let plugin = pluginSummary(pluginID: pluginID) else {
      runtimeDetail = "Plugin manifest path is unavailable."
      return
    }

    runtimeDetail = FileRevealService.revealFilePath(
      plugin.manifestPath,
      successDetail: "Revealed \(plugin.displayName) manifest."
    )
  }

  func revealPluginSourcePath(_ sourcePath: String) {
    let sourcePath = sourcePath.trimmingCharacters(in: .whitespacesAndNewlines)
    guard !sourcePath.isEmpty else {
      runtimeDetail = "Plugin source path is unavailable."
      return
    }

    runtimeDetail = FileRevealService.revealFilePath(
      sourcePath,
      successDetail: "Revealed plugin source."
    )
  }

  func isPluginCommandIssueEntry(_ entry: TimelineEntry) -> Bool {
    entry.attributes["pluginCommandStatus"] == "failed"
      || entry.attributes["pluginCommandStatus"] == "blocked"
      || entry.attributes["pluginCommandRouting"] != nil
  }

  func isPluginCommandRetryableEntry(_ entry: TimelineEntry) -> Bool {
    isPluginCommandIssueEntry(entry)
  }

  private func pluginSourcePath(from entry: TimelineEntry) -> String? {
    let explicitPath = [
      "pluginRunnerResolvedEntrypoint",
      "sourcePath",
      "pluginSourcePath",
      "pluginRunnerPluginRoot",
    ]
    .compactMap { key in
      entry.attributes[key]?.trimmingCharacters(in: .whitespacesAndNewlines)
    }
    .first { !$0.isEmpty }
    if let explicitPath {
      return explicitPath
    }

    guard let pluginID = pluginID(from: entry),
          let plugin = pluginSummary(pluginID: pluginID)
    else {
      return nil
    }

    let manifestPath = plugin.manifestPath.trimmingCharacters(in: .whitespacesAndNewlines)
    return manifestPath.isEmpty ? nil : manifestPath
  }

  private func isPluginSourceRevealEntry(_ entry: TimelineEntry) -> Bool {
    isPluginCommandIssueEntry(entry)
      || isPluginInstallIssueEntry(entry)
      || isPluginConnectorIssueEntry(entry)
      || isPluginLifecycleIssueEntry(entry)
      || isPluginCommandApprovalEntry(entry)
  }

  private func isPluginSourceRefreshEntry(_ entry: TimelineEntry) -> Bool {
    isPluginRecoveryEntry(entry)
  }

  private func isPluginRecoveryEntry(_ entry: TimelineEntry) -> Bool {
    isPluginCommandIssueEntry(entry)
      || isPluginInstallIssueEntry(entry)
      || isPluginConnectorIssueEntry(entry)
      || isPluginLifecycleIssueEntry(entry)
  }

  private func pluginID(from entry: TimelineEntry) -> String? {
    if let pluginID = entry.attributes["pluginId"]?.trimmingCharacters(in: .whitespacesAndNewlines),
       !pluginID.isEmpty
    {
      return pluginID
    }

    guard let qualifiedID = entry.attributes["commandId"]
      ?? entry.attributes["connectorId"]
    else {
      return nil
    }

    guard let separatorRange = qualifiedID.range(of: "::") else {
      return nil
    }

    let pluginID = String(qualifiedID[..<separatorRange.lowerBound])
    return pluginID.isEmpty ? nil : pluginID
  }

  private func pluginConnectorID(from entry: TimelineEntry) -> String? {
    if isPluginCommandIssueEntry(entry),
       let commandID = entry.attributes["commandId"],
       let connectorID = PluginActionPlanner.commandAuthorizationConnectorID(
         commandID: commandID,
         snapshot: pluginActionSnapshot()
       )
    {
      return connectorID
    }

    if let connectorID = entry.attributes["connectorId"]?
      .trimmingCharacters(in: .whitespacesAndNewlines),
       !connectorID.isEmpty
    {
      return connectorID
    }

    for key in ["connectorIds", "pluginRunnerConnectorId", "pluginRunnerConnectorIds"] {
      if let connectorID = singleConnectorID(from: entry.attributes[key]) {
        return connectorID
      }
    }

    return nil
  }

  private func singleConnectorID(from value: String?) -> String? {
    let connectorIDs = value?
      .split(separator: ",")
      .map { $0.trimmingCharacters(in: .whitespacesAndNewlines) }
      .filter { !$0.isEmpty } ?? []
    return connectorIDs.count == 1 ? connectorIDs[0] : nil
  }

  private func isPluginInstallIssueEntry(_ entry: TimelineEntry) -> Bool {
    switch entry.attributes["pluginInstallStatus"] {
    case "failed",
         "previewFailed",
         "inspectFailed",
         "installFailed",
         "refreshFailed",
         "blocked",
         "alreadyInstalled":
      return true
    default:
      return false
    }
  }

  private func isPluginConnectorIssueEntry(_ entry: TimelineEntry) -> Bool {
    entry.attributes["connectorStatus"] != nil
      || entry.attributes["connectorRepairHint"] != nil
  }

  private func isPluginConnectorAuthorizationEntry(_ entry: TimelineEntry) -> Bool {
    isPluginConnectorIssueEntry(entry)
      || isPluginCommandIssueEntry(entry)
  }

  private func isPluginLifecycleIssueEntry(_ entry: TimelineEntry) -> Bool {
    entry.attributes["pluginLifecycleStatus"] != nil
  }

  private func isPluginCommandApprovalEntry(_ entry: TimelineEntry) -> Bool {
    entry.kind == .approval
      && entry.attributes["action"] == "run_plugin_command"
      && entry.attributes["sourcePath"] != nil
  }
}
