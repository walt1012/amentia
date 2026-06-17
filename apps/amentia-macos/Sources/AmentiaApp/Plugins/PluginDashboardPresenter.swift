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
      return "No plugins yet."
    }

    let readyCount = readyPluginList(snapshot).count
    let invalidCount = snapshot.plugins.count - readyCount
    if invalidCount == 0 {
      return "\(readyCount) plugin\(readyCount == 1 ? "" : "s") ready"
    }

    return "\(readyCount) ready, \(invalidCount) need attention"
  }

  static func localPluginCountSummary(_ snapshot: PluginDashboardSnapshot) -> String {
    let localPlugins = snapshot.plugins.filter { $0.provenance == "local" }

    if localPlugins.isEmpty {
      return "No local plugins installed yet."
    }

    return "\(localPlugins.count) local plugin\(localPlugins.count == 1 ? "" : "s")"
  }

  static func pluginDetailSummary(_ snapshot: PluginDashboardSnapshot) -> String {
    let diagnostics = pluginLoadDiagnostics(snapshot)
    guard !snapshot.plugins.isEmpty else {
      return diagnostics
        ?? "Amentia discovers plugins from the app and your plugin folder."
    }

    let pluginDetails = snapshot.plugins
      .map { plugin in
        let capabilities = plugin.capabilitySummary.isEmpty
          ? "No declared capabilities"
          : plugin.capabilitySummary
        let validation = plugin.validationError == nil ? "ready" : "needs attention"
        let hint = plugin.validationHint.map { " | fix: \($0)" } ?? ""
        return "\(plugin.displayName) \(plugin.version) | \(PluginStatusDisplay.pluginStatus(plugin.status)) | \(plugin.sourceLabel) | \(capabilities) | \(validation)\(hint)"
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
      return "Plugin permissions are not loaded yet."
    }

    if uniquePermissions.isEmpty {
      return "\(readyPlugins.count) ready plugin\(readyPlugins.count == 1 ? "" : "s"), no extra permissions"
    }

    return "\(uniquePermissions.count) permission\(uniquePermissions.count == 1 ? "" : "s") across \(readyPlugins.count) ready plugin\(readyPlugins.count == 1 ? "" : "s")"
  }

  static func permissionDetailSummary(_ snapshot: PluginDashboardSnapshot) -> String {
    let readyPlugins = readyPluginList(snapshot)

    guard !readyPlugins.isEmpty else {
      return "Plugin permissions appear here after Amentia loads local plugins."
    }

    let uniquePermissions = Set(readyPlugins.flatMap(\.permissions))
    if uniquePermissions.isEmpty {
      return "The current ready plugins do not request extra local permissions."
    }

    return uniquePermissions
      .sorted()
      .map { permission in
        let grantingPlugins = readyPlugins
          .filter { $0.permissions.contains(permission) }
          .map(\.displayName)
          .sorted()
          .joined(separator: ", ")
        return "\(PluginPermissionDisplay.label(permission)): \(grantingPlugins)"
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

    return "\(invalidPlugins.count) plugin setup issue\(invalidPlugins.count == 1 ? "" : "s")"
  }

  static func invalidPluginDetailSummary(_ snapshot: PluginDashboardSnapshot) -> String {
    let invalidPlugins = invalidPluginList(snapshot)

    guard !invalidPlugins.isEmpty else {
      return "All discovered plugins match the current local plugin format."
    }

    return invalidPlugins
      .map { plugin in
        PluginStatusDisplay.validationIssue(plugin)
      }
      .joined(separator: "\n")
  }

  static func invalidPlugins(_ snapshot: PluginDashboardSnapshot) -> [PluginSummary] {
    invalidPluginList(snapshot)
  }

  static func registryCountSummary(_ snapshot: PluginDashboardSnapshot) -> String {
    guard let registrySummary = snapshot.registrySummary else {
      return "Plugin capabilities are not loaded yet."
    }

    let capabilityLabel = registrySummary.totalCapabilityCount == 1 ? "capability" : "capabilities"
    let pluginLabel = registrySummary.enabledPluginCount == 1 ? "plugin" : "plugins"
    return "\(registrySummary.totalCapabilityCount) \(capabilityLabel) from \(registrySummary.enabledPluginCount) enabled \(pluginLabel)"
  }

  static func registryDetailSummary(_ snapshot: PluginDashboardSnapshot) -> String {
    guard let registrySummary = snapshot.registrySummary else {
      return "Enable a ready plugin to make its capabilities available."
    }

    let kindSummary = PluginCapabilityDisplay.summary(registrySummary.capabilityCountsByKind)
    if kindSummary.isEmpty {
      return "No plugin capabilities are available yet."
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
      return "Add or enable a plugin connection to work with another app."
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
      return "Enable a plugin with actions to run reusable workflows."
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
      return "Enable a plugin with checks to verify local events."
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
      .map { "Plugin load issue: \($0)" }
      .joined(separator: "\n")
  }

  private static func connectorDetail(_ connector: PluginConnectorSummary) -> String {
    var parts = [
      "\(connector.displayName): \(PluginStatusDisplay.connectionStatus(connector.status))",
      "Authorization: \(PluginStatusDisplay.authorizationStatus(connector.authStatus, credentialPresent: connector.credentialPresent))",
      "Plugin: \(connector.pluginDisplayName)"
    ]

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
      "status: \(PluginStatusDisplay.commandStatus(command.runStatus))"
    ]

    if command.approvalRequired {
      parts.append("approval")
    }
    if !command.requiredInputFieldNames.isEmpty {
      let inputs = command.requiredInputFieldNames
        .map(PluginStatusDisplay.inputFieldLabel)
        .joined(separator: ", ")
      parts.append("input: \(inputs)")
    }
    if !command.visibleConnectorIds.isEmpty {
      parts.append("connections: \(connectorStatusList(command, connectors: connectors))")
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
          return PluginStatusDisplay.missingConnectionSummary(count: 1)
        }
        return "\(connector.displayName): \(PluginStatusDisplay.authorizationStatus(connector.authStatus, credentialPresent: connector.credentialPresent))"
      }
      .joined(separator: ", ")
  }

  private static func hookDetail(_ hook: PluginHookSummary) -> String {
    let status = hook.status == "ready"
      ? hook.event
      : "\(hook.event) | \(PluginStatusDisplay.commandStatus(hook.status))"
    if let runBlocker = hook.runBlocker {
      return "\(hook.pluginDisplayName): \(hook.title) (\(status)) | \(runBlocker)"
    }
    return "\(hook.pluginDisplayName): \(hook.title) (\(status))"
  }

}
