import Foundation

enum TimelineEventPresenter {
  static let generatingLocalResponseDetail = "Generating local response..."
  static let pendingTurnCancelledDetail = "Local execution request cancelled."
  static let runningPluginCommandDetail = "Running local plugin command..."
  static let pluginCommandNeedsExecutionContractDetail =
    "Plugin command needs a supported execution contract before it can run."
  static let pluginCommandNeedsConnectorAuthDetail =
    "Authorize the required connector before running this plugin command."
  static let pendingPluginCommandCancelledDetail = "Local plugin command cancelled."
  static let pluginLifecycleCancelledDetail = "Plugin lifecycle operation cancelled."
  static let pluginCommandBlockedDefaultDetail =
    "Local plugin command is blocked. Inspect the blocked item for the repair hint."
  static let pluginCommandFailedDetail =
    "Local plugin command failed. Inspect the failed runner item for logs."
  static let cancellingTurnDetail = "Cancelling local execution..."
  static let cancellingPluginCommandDetail = "Cancelling local plugin command..."

  static let cancelledResponsePreview = "Cancelled response"
  static let cancellingResponsePreview = "Cancelling response"
  static let cancelledPluginCommandPreview = "Cancelled plugin command"
  static let cancelledPluginLifecyclePreview = "Plugin operation cancelled"
  static let cancellingPluginCommandPreview = "Cancelling plugin command"
  static let blockedPluginCommandPreview = "Plugin command blocked"
  static let failedPluginCommandPreview = "Plugin command failed"

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
      return "Plugin command failed. \(recoveryHint)"
    }
    if let failureKind = failedItem.attributes["pluginRunnerFailureKind"],
       !failureKind.isEmpty
    {
      return "Plugin command failed: \(failureKind). Select the failed item for details."
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
      return "Plugin command blocked. \(repairHint)"
    }
    if let blocker = blockedItem.attributes["runBlocker"],
       !blocker.isEmpty
    {
      return "Plugin command blocked: \(blocker)"
    }

    return pluginCommandBlockedDefaultDetail
  }

  static func turnPreview(turnID: String, activeTurnID: String?) -> String {
    activeTurnID == nil ? "\(turnID) ready" : "Streaming response"
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
      body: "Created \(thread.title) for local work.",
      attributes: [:]
    )
  }

  static func pendingTurnCancelled() -> TimelineEntry {
    return TimelineEntryFactory.warning(
      title: "Execution Cancelled",
      body: "The pending local execution request was cancelled before it finished.",
      attributes: [:]
    )
  }

  static func turnFailed(error: Error) -> TimelineEntry {
    TimelineEntryFactory.warning(
      title: "Turn Failed",
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
      title: "Execution Cancel Failed",
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
      title: "Workspace Open Failed",
      body: error.localizedDescription,
      attributes: [:]
    )
  }

  static func workspaceOpened(_ workspace: RuntimeBridge.RuntimeWorkspace) -> TimelineEntry {
    TimelineEntryFactory.system(
      title: "Workspace Opened",
      body: "Opened \(workspace.displayName) at \(workspace.rootPath).",
      attributes: [:]
    )
  }

  static func firstRequestReady() -> TimelineEntry {
    TimelineEntryFactory.system(
      title: "Cowork Session Ready",
      body:
        "Local service, model, workspace, and session are ready. Send one short cowork prompt to finish first-use setup.",
      attributes: [
        "setup": "first-request"
      ]
    )
  }

  static func runtimeDisconnected(detail: String) -> TimelineEntry {
    TimelineEntryFactory.warning(
      title: "Local Service Disconnected",
      body: "\(detail) Use Restart Local Service to recover the session.",
      attributes: [
        "recovery": "relaunch-runtime"
      ]
    )
  }

  static func runtimeLaunchFailed(error: Error) -> TimelineEntry {
    TimelineEntryFactory.warning(
      title: "Local Service Launch Failed",
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
      body: "Saved built-in workspace note \(note.title).",
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
      title: "Plugin Install Preview Failed",
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
      "Surface: \(preview.surfaceSummary.summary)",
      preview.installRepairHint?.isEmpty == false
        ? "Repair Hint: \(preview.installRepairHint ?? "")"
        : nil,
    ]
    .compactMap { $0 }
    .joined(separator: "\n\n")

    return TimelineEntryFactory.warning(
      title: "Plugin Install Blocked",
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
      ? "No connector setup issues."
      : diagnostics.map { "Setup note: \($0)" }.joined(separator: "\n")
    var attributes = recoveryAttributes
    if attributes["pluginRefreshStatus"] == nil {
      attributes["pluginRefreshStatus"] = diagnostics.isEmpty
        ? "completed"
        : "completedWithDiagnostics"
    }
    attributes["pluginRefreshDiagnosticCount"] = "\(diagnostics.count)"

    return TimelineEntryFactory.system(
      title: diagnostics.isEmpty ? "Connectors Refreshed" : "Connectors Need Attention",
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
      body: "The current plugin lifecycle operation was cancelled before it finished.",
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
        "\(plugin.displayName) is now available in the local plugin manager.",
        "Surface: \(preview.surfaceSummary.summary)",
        "Source: \(preview.sourcePath)",
        "Installed To: \(preview.installPath)",
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
      body:
        "\(plugin.displayName) was removed from the local plugin catalog.\nRemoved Path: \(plugin.removedPath)",
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
      "Credential Binding: \(connectorCredentialBinding(connector))",
    ].joined(separator: "\n")

    return TimelineEntryFactory.system(
      title: "Connector Authorized",
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
      title: "Connector Authorization Failed",
      body: error.localizedDescription,
      attributes: attributes
    )
  }

  static func pluginConnectorCredentialCleared(
    _ connector: RuntimeBridge.RuntimePluginConnector
  ) -> TimelineEntry {
    TimelineEntryFactory.system(
      title: "Connector Credential Cleared",
      body: "\(connector.displayName) credentials were cleared from local connector state.",
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
      title: "Connector Credential Clear Failed",
      body: error.localizedDescription,
      attributes: attributes
    )
  }

  static func pluginCommandBlocked(
    _ command: PluginCommandSummary,
    detail: String?,
    input: String?
  ) -> TimelineEntry {
    let blocker = detail ?? command.runBlocker ?? "Plugin command is not ready."
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
      title: "Plugin Command Blocked",
      body: body,
      attributes: attributes
    )
  }

  static func pluginCommandCancelled() -> TimelineEntry {
    TimelineEntryFactory.warning(
      title: "Plugin Command Cancelled",
      body: "The pending local plugin command was cancelled before streaming started.",
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
      title: isBlocked ? "Plugin Command Blocked" : "Plugin Command Failed",
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
      "credentialBinding": connectorCredentialBinding(connector),
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

  private static func connectorCredentialBinding(
    _ connector: RuntimeBridge.RuntimePluginConnector
  ) -> String {
    if !connector.credentialPresent {
      return "none"
    }

    return connector.credentialSecretPresent ? "env-bound" : "marker-only"
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
