import Foundation

enum TimelineEventPresenter {
  static let generatingLocalResponseDetail = "Pith is preparing a response..."
  static let pendingTurnCancelledDetail = "Request cancelled."
  static let runningPluginCommandDetail = "Running plugin action..."
  static let pluginCommandNeedsExecutionContractDetail =
    "Plugin action needs a supported local runner before it can run."
  static let pluginCommandNeedsConnectorAuthDetail =
    "Authorize the required connection before running this action."
  static let pendingPluginCommandCancelledDetail = "Plugin action cancelled."
  static let pluginLifecycleCancelledDetail = "Plugin operation cancelled."
  static let pluginCommandBlockedDefaultDetail =
    "Plugin action needs attention. Select the item for the repair hint."
  static let pluginCommandFailedDetail =
    "Plugin action failed. Select the failed item for details."
  static let cancellingTurnDetail = "Cancelling request..."
  static let cancellingPluginCommandDetail = "Cancelling plugin action..."

  static let cancelledResponsePreview = "Cancelled response"
  static let cancellingResponsePreview = "Cancelling response"
  static let cancelledPluginCommandPreview = "Cancelled plugin action"
  static let cancelledPluginLifecyclePreview = "Plugin operation cancelled"
  static let cancellingPluginCommandPreview = "Cancelling plugin action"
  static let blockedPluginCommandPreview = "Plugin action needs attention"
  static let failedPluginCommandPreview = "Plugin action failed"

  static func pluginCommandFailureDetail(
    from items: [RuntimeBridge.RuntimeTimelineItemResult]
  ) -> String {
    guard let failedItem = items.first(where: {
      $0.attributes["pluginCommandStatus"] == "failed"
    }) else {
      return pluginCommandFailedDetail
    }

    if let recoveryHint = failedItem.attributes["pluginRunnerRecoveryHint"],
       !recoveryHint.isEmpty
    {
      return "Plugin action failed. \(recoveryHint)"
    }
    if let failureKind = failedItem.attributes["pluginRunnerFailureKind"],
       !failureKind.isEmpty
    {
      return "Plugin action failed: \(failureKind). Select the failed item for details."
    }

    return pluginCommandFailedDetail
  }

  static func pluginCommandBlockedDetail(
    from items: [RuntimeBridge.RuntimeTimelineItemResult]
  ) -> String {
    guard let blockedItem = items.first(where: {
      $0.attributes["pluginCommandStatus"] == "blocked"
    }) else {
      return pluginCommandBlockedDefaultDetail
    }

    if let repairHint = blockedItem.attributes["runRepairHint"],
       !repairHint.isEmpty
    {
      return "Plugin action needs attention. \(repairHint)"
    }
    if let blocker = blockedItem.attributes["runBlocker"],
       !blocker.isEmpty
    {
      return "Plugin action needs attention: \(blocker)"
    }

    return pluginCommandBlockedDefaultDetail
  }

  static func turnPreview(turnID: String, activeTurnID: String?) -> String {
    activeTurnID == nil ? "Response ready" : "Response in progress"
  }

  static func threadCreationFailed(error: Error) -> TimelineEntry {
    return TimelineEntryFactory.warning(
      title: "Session Creation Failed",
      body: error.localizedDescription,
      attributes: [:]
    )
  }

  static func threadCreated(_ thread: ThreadSummary) -> TimelineEntry {
    TimelineEntryFactory.system(
      title: "Session Created",
      body: "Created \(thread.title) for this project.",
      attributes: [:]
    )
  }

  static func pendingTurnCancelled() -> TimelineEntry {
    return TimelineEntryFactory.warning(
      title: "Request Cancelled",
      body: "The pending request was cancelled before it finished.",
      attributes: [:]
    )
  }

  static func turnFailed(error: Error) -> TimelineEntry {
    TimelineEntryFactory.warning(
      title: "Request Failed",
      body: error.localizedDescription,
      attributes: [:]
    )
  }

  static func approvalResponseFailed(error: Error) -> TimelineEntry {
    TimelineEntryFactory.warning(
      title: "Approval Response Failed",
      body: error.localizedDescription,
      attributes: [:]
    )
  }

  static func approvalResponseFailedDetail(error: Error) -> String {
    "Approval response failed: \(error.localizedDescription)"
  }

  static func turnCancelFailed(error: Error) -> TimelineEntry {
    TimelineEntryFactory.warning(
      title: "Cancel Failed",
      body: error.localizedDescription,
      attributes: [:]
    )
  }

  static func turnCancelFailedDetail(error: Error) -> String {
    "Cancel failed: \(error.localizedDescription)"
  }

  static func threadLoadFailed(error: Error) -> TimelineEntry {
    TimelineEntryFactory.warning(
      title: "Session Load Failed",
      body: error.localizedDescription,
      attributes: [:]
    )
  }

  static func workspaceOpenFailed(error: Error) -> TimelineEntry {
    TimelineEntryFactory.warning(
      title: "Project Open Failed",
      body: error.localizedDescription,
      attributes: [:]
    )
  }

  static func workspaceOpened(_ workspace: RuntimeBridge.RuntimeWorkspace) -> TimelineEntry {
    TimelineEntryFactory.system(
      title: "Project Opened",
      body: "Opened \(workspace.displayName) as the active project.",
      attributes: [
        "workspacePath": workspace.rootPath
      ]
    )
  }

  static func firstRequestReady() -> TimelineEntry {
    TimelineEntryFactory.system(
      title: "Cowork Session Ready",
      body:
        "Pith, the local model, project, and session are ready. Send one short cowork prompt to finish first-use setup.",
      attributes: [
        "setup": "first-request"
      ]
    )
  }

  static func runtimeDisconnected(detail: String) -> TimelineEntry {
    TimelineEntryFactory.warning(
      title: "Pith Disconnected",
      body: "\(detail) Use Restart Pith to recover the session.",
      attributes: [
        "recovery": "relaunch-runtime"
      ]
    )
  }

  static func runtimeLaunchFailed(error: Error) -> TimelineEntry {
    TimelineEntryFactory.warning(
      title: "Pith Launch Failed",
      body: error.localizedDescription,
      attributes: [:]
    )
  }

  static func localModelDownloaded(_ plan: LocalModelDownloadCompletionPlan) -> TimelineEntry {
    TimelineEntryFactory.system(
      title: "Local Model Downloaded",
      body: plan.timelineBody,
      attributes: plan.attributes
    )
  }

  static func localModelEvent(
    title: String,
    body: String,
    model: LocalModelSummary,
    kind: TimelineEntry.Kind = .system,
    attributes: [String: String] = [:]
  ) -> TimelineEntry {
    var eventAttributes = attributes
    eventAttributes["modelId"] = model.id
    eventAttributes["modelPath"] = model.installPath
    eventAttributes["modelLicense"] = model.license
    return TimelineEntryFactory.entry(
      kind: kind,
      title: title,
      body: body,
      attributes: eventAttributes
    )
  }

  static func localModelActivated(_ plan: LocalModelActivationPlan) -> TimelineEntry {
    TimelineEntryFactory.system(
      title: plan.timelineTitle,
      body: plan.timelineBody,
      attributes: plan.attributes
    )
  }

  static func memoryNoteSaved(_ note: RuntimeBridge.RuntimeMemoryNote) -> TimelineEntry {
    TimelineEntryFactory.system(
      title: "Memory Note Saved",
      body: "Saved project memory note \(note.title).",
      attributes: [
        "memoryNoteId": note.id,
        "memoryScope": note.scope,
        "memorySource": note.source,
      ]
    )
  }

  static func memoryNoteFailed(error: Error) -> TimelineEntry {
    TimelineEntryFactory.warning(
      title: "Memory Note Failed",
      body: error.localizedDescription,
      attributes: [:]
    )
  }

  static func pluginInstallPreviewFailed(
    error: Error,
    repairHint: String,
    sourcePath: String
  ) -> TimelineEntry {
    let body = repairHint.isEmpty
      ? error.localizedDescription
      : "\(error.localizedDescription)\n\nRepair Hint: \(repairHint)"

    return TimelineEntryFactory.warning(
      title: "Plugin Preview Failed",
      body: body,
      attributes: pluginInstallFailureAttributes(
        error,
        fallbackStatus: "previewFailed",
        sourcePath: sourcePath,
        repairHint: repairHint
      )
    )
  }

  static func pluginInstallBlocked(preview: PluginInstallPreview) -> TimelineEntry {
    let blocker = preview.installBlocker ?? "Plugin cannot be installed yet."
    let body = [
      blocker,
      "Capabilities: \(pluginCapabilitySummary(preview.capabilities))",
      "Permissions: \(PluginPermissionDisplay.summary(preview.permissions))",
      preview.installRepairHint?.isEmpty == false
        ? "Repair Hint: \(preview.installRepairHint ?? "")"
        : nil,
    ]
    .compactMap { $0 }
    .joined(separator: "\n\n")

    return TimelineEntryFactory.warning(
      title: "Plugin Install Needs Attention",
      body: body,
      attributes: [
        "pluginId": preview.pluginID,
        "pluginInstallStatus": preview.installStatus,
        "pluginSourcePath": preview.sourcePath,
        "sourcePath": preview.sourcePath,
        "pluginInstallPath": preview.installPath,
      ]
    )
  }

  static func pluginCatalogRefreshed(
    pluginSummary: String,
    surfaceSummary: String,
    diagnostics: [String],
    recoveryAttributes: [String: String]
  ) -> TimelineEntry {
    let setupBody = diagnostics.isEmpty
      ? "No plugin setup issues."
      : diagnostics.map { "Setup note: \($0)" }.joined(separator: "\n")
    var attributes = recoveryAttributes
    if attributes["pluginRefreshStatus"] == nil {
      attributes["pluginRefreshStatus"] = diagnostics.isEmpty
        ? "completed"
        : "completedWithDiagnostics"
    }
    attributes["pluginRefreshDiagnosticCount"] = "\(diagnostics.count)"

    return TimelineEntryFactory.system(
      title: diagnostics.isEmpty ? "Plugins Refreshed" : "Plugins Need Attention",
      body: [
        pluginSummary,
        surfaceSummary,
        setupBody,
      ].joined(separator: "\n"),
      attributes: attributes
    )
  }

  static func pluginLifecycleCancelled() -> TimelineEntry {
    TimelineEntryFactory.warning(
      title: "Plugin Operation Cancelled",
      body: "The current plugin operation was cancelled before it finished.",
      attributes: [
        "pluginLifecycleOperation": "lifecycle",
        "pluginLifecycleStatus": "cancelled",
      ]
    )
  }

  static func pluginInstalled(
    _ plugin: RuntimeBridge.RuntimePlugin,
    preview: PluginInstallPreview
  ) -> TimelineEntry {
    TimelineEntryFactory.system(
      title: "Plugin Installed",
      body: [
        "\(plugin.displayName) is now available.",
        "Capabilities: \(pluginCapabilitySummary(preview.capabilities))",
        "Permissions: \(PluginPermissionDisplay.summary(preview.permissions))",
      ].joined(separator: "\n"),
      attributes: [
        "pluginId": plugin.id,
        "pluginStatus": plugin.status,
        "pluginManifestPath": plugin.manifestPath,
        "pluginSourcePath": preview.sourcePath,
        "sourcePath": preview.sourcePath,
        "pluginInstallPath": preview.installPath,
      ]
    )
  }

  static func pluginInstallFailed(
    error: Error,
    repairHint: String,
    sourcePath: String?
  ) -> TimelineEntry {
    let body = repairHint.isEmpty
      ? error.localizedDescription
      : "\(error.localizedDescription)\n\nRepair Hint: \(repairHint)"

    return TimelineEntryFactory.warning(
      title: "Plugin Install Failed",
      body: body,
      attributes: pluginInstallFailureAttributes(
        error,
        fallbackStatus: "failed",
        sourcePath: sourcePath,
        repairHint: repairHint
      )
    )
  }

  static func pluginUpdated(
    _ plugin: RuntimeBridge.RuntimePlugin,
    enabled: Bool
  ) -> TimelineEntry {
    TimelineEntryFactory.system(
      title: enabled ? "Plugin Enabled" : "Plugin Disabled",
      body: "\(plugin.displayName) is now \(enabled ? "enabled" : "disabled").",
      attributes: [
        "pluginId": plugin.id,
        "pluginStatus": plugin.status,
      ]
    )
  }

  static func pluginUpdateFailed(pluginID: String, enabled: Bool, error: Error) -> TimelineEntry {
    TimelineEntryFactory.warning(
      title: "Plugin Update Failed",
      body: error.localizedDescription,
      attributes: pluginLifecycleFailureAttributes(
        error,
        fallbackOperation: enabled ? "enable" : "disable",
        fallbackStatus: "failed",
        pluginID: pluginID
      )
    )
  }

  static func pluginRemoved(_ plugin: RuntimeBridge.RuntimePluginRemoval) -> TimelineEntry {
    TimelineEntryFactory.system(
      title: "Plugin Removed",
      body: "\(plugin.displayName) was removed.",
      attributes: [
        "pluginId": plugin.pluginID,
        "removedPath": plugin.removedPath,
      ]
    )
  }

  static func pluginRemovalFailed(pluginID: String, error: Error) -> TimelineEntry {
    TimelineEntryFactory.warning(
      title: "Plugin Removal Failed",
      body: error.localizedDescription,
      attributes: pluginLifecycleFailureAttributes(
        error,
        fallbackOperation: "remove",
        fallbackStatus: "failed",
        pluginID: pluginID
      )
    )
  }

  static func pluginConnectorAuthorized(
    _ connector: RuntimeBridge.RuntimePluginConnector
  ) -> TimelineEntry {
    let body = [
      "\(connector.displayName) is ready for \(connector.service) through "
        + "\(connector.pluginDisplayName).",
      "Authorization: \(connectorAuthorizationSummary(connector))",
    ].joined(separator: "\n")

    return TimelineEntryFactory.system(
      title: "Connection Authorized",
      body: body,
      attributes: pluginConnectorCredentialAttributes(connector)
    )
  }

  static func pluginConnectorAuthFailed(
    connectorID: String,
    error: Error
  ) -> TimelineEntry {
    let attributes = pluginConnectorFailureAttributes(
      error,
      fallbackConnectorID: connectorID
    )

    return TimelineEntryFactory.warning(
      title: "Connection Authorization Failed",
      body: error.localizedDescription,
      attributes: attributes
    )
  }

  static func pluginConnectorCredentialCleared(
    _ connector: RuntimeBridge.RuntimePluginConnector
  ) -> TimelineEntry {
    TimelineEntryFactory.system(
      title: "Connection Credential Cleared",
      body: "\(connector.displayName) credentials were cleared from local plugin state.",
      attributes: pluginConnectorCredentialAttributes(connector)
    )
  }

  static func pluginConnectorCredentialClearFailed(
    connectorID: String,
    error: Error
  ) -> TimelineEntry {
    let attributes = pluginConnectorFailureAttributes(
      error,
      fallbackConnectorID: connectorID
    )

    return TimelineEntryFactory.warning(
      title: "Connection Credential Clear Failed",
      body: error.localizedDescription,
      attributes: attributes
    )
  }

  static func pluginCommandBlocked(
    _ command: PluginCommandSummary,
    detail: String?,
    input: String?
  ) -> TimelineEntry {
    let blocker = detail ?? command.runBlocker ?? "Plugin action is not ready."
    let repair = command.runRepairHint?.trimmingCharacters(in: .whitespacesAndNewlines)
    let body = repair?.isEmpty == false
      ? "\(blocker)\n\nRepair Hint: \(repair ?? "")"
      : blocker

    var attributes = [
      "pluginCommandStatus": "blocked",
      "commandId": command.id,
      "pluginId": command.pluginID,
      "pluginDisplayName": command.pluginDisplayName,
      "sourcePath": command.sourcePath,
      "runStatus": command.runStatus,
    ]

    if let runBlocker = command.runBlocker {
      attributes["runBlocker"] = runBlocker
    }
    if let runRepairHint = command.runRepairHint {
      attributes["runRepairHint"] = runRepairHint
    }
    if let executionKind = command.executionKind {
      attributes["executionKind"] = executionKind
    }
    let connectorIDs = command.requiredConnectorIds.isEmpty
      ? command.declaredConnectorIds
      : command.requiredConnectorIds
    if !connectorIDs.isEmpty {
      attributes["connectorIds"] = connectorIDs.joined(separator: ", ")
      if connectorIDs.count == 1 {
        attributes["connectorId"] = connectorIDs[0]
      }
    }
    if let input {
      attributes["commandInput"] = input
    }

    return TimelineEntryFactory.warning(
      title: "Plugin Action Needs Attention",
      body: body,
      attributes: attributes
    )
  }

  static func pluginCommandCancelled() -> TimelineEntry {
    TimelineEntryFactory.warning(
      title: "Plugin Action Cancelled",
      body: "The pending plugin action was cancelled before streaming started.",
      attributes: [:]
    )
  }

  static func pluginCommandFailed(error: Error) -> TimelineEntry {
    var attributes = pluginCommandFailureAttributes(error)
    let isBlocked = attributes["runStatus"] != nil
      || attributes["runBlocker"] != nil
      || attributes["runRepairHint"] != nil
    if isBlocked {
      attributes["pluginCommandStatus"] = "blocked"
    } else if !attributes.isEmpty {
      attributes["pluginCommandStatus"] = "failed"
    }

    return TimelineEntryFactory.warning(
      title: isBlocked ? "Plugin Action Needs Attention" : "Plugin Action Failed",
      body: error.localizedDescription,
      attributes: attributes
    )
  }

  private static func pluginConnectorCredentialAttributes(
    _ connector: RuntimeBridge.RuntimePluginConnector
  ) -> [String: String] {
    var attributes = [
      "connectorId": connector.connectorID,
      "pluginId": connector.pluginID,
      "pluginDisplayName": connector.pluginDisplayName,
      "connectorService": connector.service,
      "authStatus": connector.authStatus,
      "credentialPresent": "\(connector.credentialPresent)",
      "authorizationSummary": connectorAuthorizationSummary(connector),
    ]

    if let authType = connector.authType {
      attributes["authType"] = authType
    }
    if let credentialStore = connector.credentialStore {
      attributes["credentialStore"] = credentialStore
    }
    if let credentialProvider = connector.credentialProvider {
      attributes["credentialProvider"] = credentialProvider
    }
    if let credentialLabel = connector.credentialLabel {
      attributes["credentialLabel"] = credentialLabel
    }
    if let authorizedAt = connector.authorizedAt {
      attributes["authorizedAt"] = "\(authorizedAt)"
    }
    if let credentialUpdatedAt = connector.credentialUpdatedAt {
      attributes["credentialUpdatedAt"] = "\(credentialUpdatedAt)"
    }

    return attributes
  }

  private static func connectorAuthorizationSummary(
    _ connector: RuntimeBridge.RuntimePluginConnector
  ) -> String {
    if !connector.credentialPresent {
      return "not saved"
    }

    return connector.credentialSecretPresent ? "saved locally" : "authorized without a secret"
  }

  private static func pluginCapabilitySummary(_ capabilities: [String]) -> String {
    let summary = PluginCapabilityDisplay.summary(capabilities)
    return summary.isEmpty ? "No declared capabilities" : summary
  }

  private static func recoveryAttributes(_ error: Error) -> [String: String] {
    guard let runtimeError = error as? RuntimeBridge.RuntimeError else {
      return [:]
    }
    return runtimeError.recoveryAttributes
  }

  private static func pluginInstallFailureAttributes(
    _ error: Error,
    fallbackStatus: String,
    sourcePath: String?,
    repairHint: String
  ) -> [String: String] {
    var attributes = recoveryAttributes(error)
    if attributes["pluginInstallStatus"] == nil {
      attributes["pluginInstallStatus"] = fallbackStatus
    }
    if attributes["installBlocker"] == nil {
      attributes["installBlocker"] = error.localizedDescription
    }
    if attributes["installRepairHint"] == nil && !repairHint.isEmpty {
      attributes["installRepairHint"] = repairHint
    }
    if let sourcePath, !sourcePath.isEmpty {
      if attributes["pluginSourcePath"] == nil {
        attributes["pluginSourcePath"] = sourcePath
      }
      if attributes["sourcePath"] == nil {
        attributes["sourcePath"] = sourcePath
      }
    }
    return attributes
  }

  private static func pluginLifecycleFailureAttributes(
    _ error: Error,
    fallbackOperation: String,
    fallbackStatus: String,
    pluginID: String
  ) -> [String: String] {
    var attributes = recoveryAttributes(error)
    if attributes["pluginId"] == nil {
      attributes["pluginId"] = pluginID
    }
    if attributes["pluginLifecycleOperation"] == nil {
      attributes["pluginLifecycleOperation"] = fallbackOperation
    }
    if attributes["pluginLifecycleStatus"] == nil {
      attributes["pluginLifecycleStatus"] = fallbackStatus
    }
    if attributes["lifecycleBlocker"] == nil {
      attributes["lifecycleBlocker"] = error.localizedDescription
    }
    return attributes
  }

  private static func pluginCommandFailureAttributes(_ error: Error) -> [String: String] {
    recoveryAttributes(error)
  }

  private static func pluginConnectorFailureAttributes(
    _ error: Error,
    fallbackConnectorID: String
  ) -> [String: String] {
    var attributes = pluginCommandFailureAttributes(error)
    if attributes["connectorId"] == nil {
      attributes["connectorId"] = fallbackConnectorID
    }
    if attributes["connectorStatus"] == nil {
      attributes["connectorStatus"] = "failed"
    }
    if attributes["pluginId"] == nil,
       let connectorID = attributes["connectorId"],
       let pluginID = pluginID(fromQualifiedID: connectorID) {
      attributes["pluginId"] = pluginID
    }
    return attributes
  }

  private static func pluginID(fromQualifiedID qualifiedID: String) -> String? {
    guard let separatorRange = qualifiedID.range(of: "::") else {
      return nil
    }

    let pluginID = String(qualifiedID[..<separatorRange.lowerBound])
    return pluginID.isEmpty ? nil : pluginID
  }
}
