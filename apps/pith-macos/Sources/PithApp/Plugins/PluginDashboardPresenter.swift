import Foundation

struct PluginDashboardSnapshot {
  let plugins: [PluginSummary]
  let registrySummary: PluginCapabilityRegistrySummary?
  let capabilities: [PluginCapabilitySummary]
  let connectors: [PluginConnectorSummary]
  let commands: [PluginCommandSummary]
  let hooks: [PluginHookSummary]
  let diagnostics: [String]
  let refreshRecoveryAttributes: [String: String]
  let hasLifecycleOperation: Bool
}

enum PluginDashboardPresenter {
  static func pluginCountSummary(_ snapshot: PluginDashboardSnapshot) -> String {
    if snapshot.plugins.isEmpty {
      return "No local connectors yet."
    }

    let readyCount = readyPluginList(snapshot).count
    let invalidCount = snapshot.plugins.count - readyCount
    if invalidCount == 0 {
      return "\(readyCount) local \(readyCount == 1 ? "connector" : "connectors") ready"
    }

    return "\(readyCount) ready, \(invalidCount) need attention"
  }

  static func localPluginCountSummary(_ snapshot: PluginDashboardSnapshot) -> String {
    let localPlugins = snapshot.plugins.filter { $0.provenance == "local" }

    if localPlugins.isEmpty {
      return "No local connectors installed yet."
    }

    return "\(localPlugins.count) local connector\(localPlugins.count == 1 ? "" : "s")"
  }

  static func pluginDetailSummary(_ snapshot: PluginDashboardSnapshot) -> String {
    let diagnostics = pluginLoadDiagnostics(snapshot)
    guard !snapshot.plugins.isEmpty else {
      return diagnostics
        ?? "Pith discovers local connectors from the app and your connector folder."
    }

    let pluginDetails = snapshot.plugins
      .map { plugin in
        let capabilities = plugin.capabilities.isEmpty ? "none" : plugin.capabilities.joined(separator: ", ")
        let validation = plugin.validationError ?? "ok"
        let hint = plugin.validationHint.map { " | fix: \($0)" } ?? ""
        return "\(plugin.displayName) \(plugin.version) | \(plugin.status) | \(plugin.provenance) | can use: \(capabilities) | check: \(validation)\(hint)"
      }
      .joined(separator: "\n")

    guard let diagnostics else {
      return pluginDetails
    }

    return "\(diagnostics)\n\(pluginDetails)"
  }

  static func catalogPreview(_ snapshot: PluginDashboardSnapshot) -> [PluginSummary] {
    snapshot.plugins
  }

  static func permissionCountSummary(_ snapshot: PluginDashboardSnapshot) -> String {
    let readyPlugins = readyPluginList(snapshot)
    let uniquePermissions = Set(readyPlugins.flatMap(\.permissions))

    guard !readyPlugins.isEmpty else {
      return "Connector permissions are not loaded yet."
    }

    if uniquePermissions.isEmpty {
      return "\(readyPlugins.count) ready connector\(readyPlugins.count == 1 ? "" : "s"), no extra permissions"
    }

    return "\(uniquePermissions.count) permission\(uniquePermissions.count == 1 ? "" : "s") across \(readyPlugins.count) ready connector\(readyPlugins.count == 1 ? "" : "s")"
  }

  static func permissionDetailSummary(_ snapshot: PluginDashboardSnapshot) -> String {
    let readyPlugins = readyPluginList(snapshot)

    guard !readyPlugins.isEmpty else {
      return "Connector permissions appear here after Pith loads local connectors."
    }

    let uniquePermissions = Set(readyPlugins.flatMap(\.permissions))
    if uniquePermissions.isEmpty {
      return "The current ready connectors do not request extra local permissions."
    }

    return uniquePermissions
      .sorted()
      .map { permission in
        let grantingPlugins = readyPlugins
          .filter { $0.permissions.contains(permission) }
          .map(\.displayName)
          .sorted()
          .joined(separator: ", ")
        return "\(permission): \(grantingPlugins)"
      }
      .joined(separator: "\n")
  }

  static func permissionPreview(_ snapshot: PluginDashboardSnapshot) -> [PluginSummary] {
    readyPluginList(snapshot)
  }

  static func invalidPluginCountSummary(_ snapshot: PluginDashboardSnapshot) -> String {
    let invalidPlugins = invalidPluginList(snapshot)

    if invalidPlugins.isEmpty {
      return "No Setup Issues"
    }

    return "\(invalidPlugins.count) connector setup issue\(invalidPlugins.count == 1 ? "" : "s")"
  }

  static func invalidPluginDetailSummary(_ snapshot: PluginDashboardSnapshot) -> String {
    let invalidPlugins = invalidPluginList(snapshot)

    guard !invalidPlugins.isEmpty else {
      return "All discovered connectors match the current local connector format."
    }

    return invalidPlugins
      .map { plugin in
        let hint = plugin.validationHint.map { " Repair hint: \($0)" } ?? ""
        return "\(plugin.displayName): \(plugin.validationError ?? "Unknown validation error")\(hint)"
      }
      .joined(separator: "\n")
  }

  static func invalidPlugins(_ snapshot: PluginDashboardSnapshot) -> [PluginSummary] {
    invalidPluginList(snapshot)
  }

  static func registryCountSummary(_ snapshot: PluginDashboardSnapshot) -> String {
    guard let registrySummary = snapshot.registrySummary else {
      return "Connection capabilities are not loaded yet."
    }

    let capabilityLabel = registrySummary.totalCapabilityCount == 1 ? "capability" : "capabilities"
    let connectorLabel = registrySummary.enabledPluginCount == 1 ? "connector" : "connectors"
    return "\(registrySummary.totalCapabilityCount) \(capabilityLabel) from \(registrySummary.enabledPluginCount) enabled \(connectorLabel)"
  }

  static func registryDetailSummary(_ snapshot: PluginDashboardSnapshot) -> String {
    guard let registrySummary = snapshot.registrySummary else {
      return "Enable a ready connector to make its actions available."
    }

    let kindSummary = registrySummary.capabilityCountsByKind
      .sorted(by: { $0.key < $1.key })
      .map { "\($0.key): \($0.value)" }
      .joined(separator: " | ")
    if kindSummary.isEmpty {
      return "No connector actions are available yet."
    }

    return kindSummary
  }

  static func capabilityPreview(_ snapshot: PluginDashboardSnapshot) -> [PluginCapabilitySummary] {
    Array(snapshot.capabilities.prefix(6))
  }

  static func connectorCountSummary(_ snapshot: PluginDashboardSnapshot) -> String {
    if snapshot.connectors.isEmpty {
      return "No connections yet"
    }

    let readyCount = snapshot.connectors.filter { $0.status == "ready" }.count
    let needsAuthCount = snapshot.connectors.filter { $0.authStatus == "needsAuth" }.count
    let authorizedCount = snapshot.connectors.filter { $0.credentialPresent }.count
    var parts = [
      "\(snapshot.connectors.count) connection\(snapshot.connectors.count == 1 ? "" : "s")"
    ]
    if readyCount > 0 {
      parts.append("\(readyCount) ready")
    }
    if needsAuthCount > 0 {
      parts.append("\(needsAuthCount) need sign in")
    }
    if authorizedCount > 0 {
      parts.append("\(authorizedCount) authorized")
    }
    return parts.joined(separator: " | ")
  }

  static func connectorDetailSummary(_ snapshot: PluginDashboardSnapshot) -> String {
    guard !snapshot.connectors.isEmpty else {
      return "Add or enable a local connector to work with another app."
    }

    return snapshot.connectors
      .map { connectorDetail($0) }
      .joined(separator: "\n")
  }

  static func connectorPreview(_ snapshot: PluginDashboardSnapshot) -> [PluginConnectorSummary] {
    Array(snapshot.connectors.prefix(6))
  }

  static func commandCountSummary(_ snapshot: PluginDashboardSnapshot) -> String {
    if snapshot.commands.isEmpty {
      return "No actions yet"
    }

    let readyCount = snapshot.commands.filter { $0.runStatus == "ready" }.count
    let blockedCount = snapshot.commands.count - readyCount
    let approvalCount = snapshot.commands.filter { $0.approvalRequired }.count
    var parts = [
      "\(snapshot.commands.count) action\(snapshot.commands.count == 1 ? "" : "s")"
    ]
    if readyCount > 0 {
      parts.append("\(readyCount) ready")
    }
    if blockedCount > 0 {
      parts.append("\(blockedCount) blocked")
    }
    if approvalCount > 0 {
      parts.append("\(approvalCount) approval gated")
    }
    return parts.joined(separator: " | ")
  }

  static func commandDetailSummary(_ snapshot: PluginDashboardSnapshot) -> String {
    guard !snapshot.commands.isEmpty else {
      return "Enable a connector with actions to run reusable local workflows."
    }

    return snapshot.commands
      .map { command in
        commandDetail(command, connectors: snapshot.connectors)
      }
      .joined(separator: "\n")
  }

  static func commandPreview(_ snapshot: PluginDashboardSnapshot) -> [PluginCommandSummary] {
    snapshot.commands
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

  static func hookCountSummary(_ snapshot: PluginDashboardSnapshot) -> String {
    if snapshot.hooks.isEmpty {
      return "No checks yet"
    }

    return "\(snapshot.hooks.count) check\(snapshot.hooks.count == 1 ? "" : "s")"
  }

  static func hookDetailSummary(_ snapshot: PluginDashboardSnapshot) -> String {
    guard !snapshot.hooks.isEmpty else {
      return "Enable a connector with checks to verify local events."
    }

    return snapshot.hooks.map(hookDetail).joined(separator: "\n")
  }

  static func hookPreview(_ snapshot: PluginDashboardSnapshot) -> [PluginHookSummary] {
    snapshot.hooks
  }

  private static func readyPluginList(_ snapshot: PluginDashboardSnapshot) -> [PluginSummary] {
    snapshot.plugins.filter { $0.status == "ready" }
  }

  private static func invalidPluginList(_ snapshot: PluginDashboardSnapshot) -> [PluginSummary] {
    snapshot.plugins.filter { $0.status != "ready" }
  }

  private static func pluginLoadDiagnostics(_ snapshot: PluginDashboardSnapshot) -> String? {
    guard !snapshot.diagnostics.isEmpty else {
      return nil
    }

    return snapshot.diagnostics
      .map { "Connector load issue: \($0)" }
      .joined(separator: "\n")
  }

  private static func connectorDetail(_ connector: PluginConnectorSummary) -> String {
    var parts = [
      "\(connector.displayName): \(connector.status)",
      "connection: \(connector.authStatus)",
      "source: \(connector.pluginDisplayName)"
    ]

    if connector.authRequired {
      parts.append("store: \(connector.credentialStore ?? "unknown")")
      if connector.credentialPresent {
        let binding = connector.credentialSecretPresent ? "env-bound" : "marker-only"
        parts.append("credential: \(binding)")
      } else {
        parts.append("fix: authorize connection")
      }
    }
    if !connector.workflows.isEmpty {
      let workflowLabels = connector.workflows
        .map(\.workflowLabel)
        .joined(separator: ", ")
      parts.append("workflows: \(workflowLabels)")
    }

    return parts.joined(separator: " | ")
  }

  private static func commandDetail(
    _ command: PluginCommandSummary,
    connectors: [PluginConnectorSummary]
  ) -> String {
    var parts = [
      "\(command.pluginDisplayName): \(command.title)",
      "status: \(command.runStatus)",
      "run: \(command.explicitTurnRoute)"
    ]

    if command.approvalRequired {
      parts.append("approval")
    }
    if !command.requiredInputFieldNames.isEmpty {
      parts.append("input: \(command.requiredInputFieldNames.joined(separator: ", "))")
    }
    if !command.visibleConnectorIds.isEmpty {
      parts.append("connectors: \(connectorStatusList(command, connectors: connectors))")
    }
    if let runBlocker = command.runBlocker, command.runStatus != "ready" {
      parts.append("blocked: \(runBlocker)")
    }
    if let repairHint = command.runRepairHint, command.runStatus != "ready" {
      parts.append("fix: \(repairHint)")
    }

    return parts.joined(separator: " | ")
  }

  private static func connectorStatusList(
    _ command: PluginCommandSummary,
    connectors: [PluginConnectorSummary]
  ) -> String {
    command.visibleConnectorIds
      .map { connectorID in
        guard let connector = connectors.first(where: { $0.id == connectorID }) else {
          return "\(connectorID): missing"
        }
        return "\(connector.displayName): \(displayConnectionStatus(connector.authStatus))"
      }
      .joined(separator: ", ")
  }

  private static func hookDetail(_ hook: PluginHookSummary) -> String {
    let status = hook.status == "ready" ? hook.event : "\(hook.event) | \(hook.status)"
    if let runBlocker = hook.runBlocker {
      return "\(hook.pluginDisplayName): \(hook.title) (\(status)) | \(runBlocker)"
    }
    return "\(hook.pluginDisplayName): \(hook.title) (\(status))"
  }

  private static func displayConnectionStatus(_ status: String) -> String {
    switch status {
    case "ready":
      return "ready"
    case "needsAuth":
      return "needs sign in"
    default:
      return status
    }
  }
}
