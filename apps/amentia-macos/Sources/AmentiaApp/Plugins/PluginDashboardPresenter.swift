enum PluginDashboardPresenter {
  private static let previewLimit = 6

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
        let hasValidationIssue = plugin.validationError != nil || plugin.validationHint != nil
        let validation = hasValidationIssue ? "needs attention" : "ready"
        let hint = plugin.validationHint
          .map { " | Fix: \(PluginValidationCopy.userFacingRepairHint($0))" } ?? ""
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
      let pluginLabel = pluralName(readyPlugins.count, "plugin")
      return "\(readyPlugins.count) ready \(pluginLabel), no extra permissions"
    }

    let permissionSummary = plural(uniquePermissions.count, "permission")
    let pluginLabel = pluralName(readyPlugins.count, "plugin")
    return "\(permissionSummary) across \(readyPlugins.count) ready \(pluginLabel)"
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

    return "\(invalidPlugins.count) \(pluralName(invalidPlugins.count, "plugin setup issue"))"
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
    preview(snapshot.capabilities)
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

  static func connectorDetailSummary(_ snapshot: PluginDashboardSnapshot) -> String {
    guard !snapshot.connectors.isEmpty else {
      return "Add or enable a plugin connection to work with another app."
    }

    return snapshot.connectors
      .map { connectorDetail($0) }
      .joined(separator: "\n")
  }

  static func connectorPreview(_ snapshot: PluginDashboardSnapshot) -> [PluginConnectorSummary] {
    preview(snapshot.connectors)
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

  static func hookCountSummary(_ snapshot: PluginDashboardSnapshot) -> String {
    if snapshot.hooks.isEmpty {
      return "No checks yet"
    }

    return plural(snapshot.hooks.count, "check")
  }

  static func hookDetailSummary(_ snapshot: PluginDashboardSnapshot) -> String {
    guard !snapshot.hooks.isEmpty else {
      return "Enable a plugin with checks to verify local activity."
    }

    return snapshot.hooks.map(hookDetail).joined(separator: "\n")
  }

  static func hookPreview(_ snapshot: PluginDashboardSnapshot) -> [PluginHookSummary] {
    preview(snapshot.hooks)
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

  static func skillDetailSummary(_ snapshot: PluginDashboardSnapshot) -> String {
    guard !snapshot.skills.isEmpty else {
      return "Enable a plugin with skills to review local instructions before use."
    }

    return preview(snapshot.skills).map(skillDetail).joined(separator: "\n")
  }

  static func skillPreview(_ snapshot: PluginDashboardSnapshot) -> [PluginSkillSummary] {
    preview(snapshot.skills)
  }

  private static func preview<Value>(_ values: [Value]) -> [Value] {
    Array(values.prefix(previewLimit))
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
      "Authorization: \(connectorAuthorizationStatus(connector))",
      "Plugin: \(connector.pluginDisplayName)"
    ]

    if !connector.workflows.isEmpty {
      let workflowLabels = connector.workflows
        .map(\.workflowLabel)
        .joined(separator: ", ")
      parts.append("Workflows: \(workflowLabels)")
    }

    return parts.joined(separator: " | ")
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

  private static func commandDetail(
    _ command: PluginCommandSummary,
    connectors: [PluginConnectorSummary]
  ) -> String {
    var parts = [
      "\(command.pluginDisplayName): \(command.title)",
      "Status: \(PluginStatusDisplay.commandStatus(command.runStatus))"
    ]

    if command.approvalRequired {
      parts.append("Needs approval")
    }
    if !command.requiredInputFieldNames.isEmpty {
      let inputs = command.requiredInputFieldNames
        .map(PluginStatusDisplay.inputFieldLabel)
        .joined(separator: ", ")
      parts.append("Input: \(inputs)")
    }
    if !command.visibleConnectorIds.isEmpty {
      parts.append("Connections: \(connectorStatusList(command, connectors: connectors))")
    }
    if let runBlocker = command.runBlocker, command.runStatus != "ready" {
      parts.append("Blocked: \(runBlocker)")
    }
    if let repairHint = command.runRepairHint, command.runStatus != "ready" {
      parts.append("Fix: \(repairHint)")
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
        return "\(connector.displayName): \(connectorAuthorizationStatus(connector))"
      }
      .joined(separator: ", ")
  }

  private static func hookDetail(_ hook: PluginHookSummary) -> String {
    var status = PluginHookDisplay.eventDetail(hook)
    if hook.status != "ready" {
      status += " | \(PluginStatusDisplay.commandStatus(hook.status))"
    }

    if let runBlocker = hook.runBlocker {
      return "\(hook.pluginDisplayName): \(hook.title) (\(status)) | \(runBlocker)"
    }
    return "\(hook.pluginDisplayName): \(hook.title) (\(status))"
  }

  private static func skillDetail(_ skill: PluginSkillSummary) -> String {
    var parts = [
      "\(skill.pluginDisplayName): \(skill.description)",
      "Status: \(PluginStatusDisplay.skillStatus(skill.status))"
    ]
    if let preview = PluginSkillDisplay.previewLine(skill.preview) {
      parts.append("Preview: \(preview)")
    }
    if let runBlocker = skill.runBlocker {
      parts.append("Blocked: \(runBlocker)")
    }
    if let runRepairHint = skill.runRepairHint {
      parts.append("Fix: \(runRepairHint)")
    }
    return parts.joined(separator: " | ")
  }

}
