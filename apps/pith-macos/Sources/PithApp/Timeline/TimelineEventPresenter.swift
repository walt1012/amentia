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
  static let pluginCommandFailedDetail =
    "Local plugin command failed. Inspect the failed runner item for logs."
  static let cancellingTurnDetail = "Cancelling local execution..."

  static let cancelledResponsePreview = "Cancelled response"
  static let cancellingResponsePreview = "Cancelling response"
  static let cancelledPluginCommandPreview = "Cancelled plugin command"
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

  static func turnPreview(turnID: String, activeTurnID: String?) -> String {
    activeTurnID == nil ? "\(turnID) ready" : "Streaming response"
  }

  static func threadCreationFailed(error: Error) -> TimelineEntry {
    TimelineEntryFactory.warning(
      title: "Thread Creation Failed",
      body: error.localizedDescription,
      attributes: [:]
    )
  }

  static func threadCreated(_ thread: ThreadSummary) -> TimelineEntry {
    TimelineEntryFactory.system(
      title: "Thread Created",
      body: "Created \(thread.title) in the local runtime.",
      attributes: [:]
    )
  }

  static func pendingTurnCancelled() -> TimelineEntry {
    TimelineEntryFactory.warning(
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
      title: "Thread Load Failed",
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
      title: "First Request Ready",
      body:
        "Runtime, local model, workspace, and thread are ready. Send one short local request to finish first-use setup.",
      attributes: [
        "setup": "first-request"
      ]
    )
  }

  static func runtimeDisconnected(detail: String) -> TimelineEntry {
    TimelineEntryFactory.warning(
      title: "Runtime Disconnected",
      body: "\(detail) Use Relaunch Runtime to recover the local session.",
      attributes: [
        "recovery": "relaunch-runtime"
      ]
    )
  }

  static func runtimeLaunchFailed(error: Error) -> TimelineEntry {
    TimelineEntryFactory.warning(
      title: "Runtime Launch Failed",
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
      attributes: [
        "pluginInstallStatus": "previewFailed",
        "pluginSourcePath": sourcePath,
        "sourcePath": sourcePath,
      ]
    )
  }

  static func pluginInstallBlocked(preview: PluginInstallPreview) -> TimelineEntry {
    let blocker = preview.installBlocker ?? "Plugin cannot be installed yet."
    let body = preview.installRepairHint?.isEmpty == false
      ? "\(blocker)\n\nRepair Hint: \(preview.installRepairHint ?? "")"
      : blocker

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

  static func pluginInstalled(
    _ plugin: RuntimeBridge.RuntimePlugin,
    preview: PluginInstallPreview
  ) -> TimelineEntry {
    TimelineEntryFactory.system(
      title: "Plugin Installed",
      body:
        "\(plugin.displayName) is now available in the local plugin manager.\nSource: \(preview.sourcePath)\nInstalled To: \(preview.installPath)",
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
    var attributes = [
      "pluginInstallStatus": "failed"
    ]
    if let sourcePath {
      attributes["pluginSourcePath"] = sourcePath
      attributes["sourcePath"] = sourcePath
    }

    return TimelineEntryFactory.warning(
      title: "Plugin Install Failed",
      body: body,
      attributes: attributes
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

  static func pluginUpdateFailed(pluginID: String, error: Error) -> TimelineEntry {
    TimelineEntryFactory.warning(
      title: "Plugin Update Failed",
      body: error.localizedDescription,
      attributes: [
        "pluginId": pluginID
      ]
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
      attributes: [
        "pluginId": pluginID
      ]
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
    TimelineEntryFactory.warning(
      title: "Connector Authorization Failed",
      body: error.localizedDescription,
      attributes: [
        "connectorId": connectorID
      ]
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
    TimelineEntryFactory.warning(
      title: "Connector Credential Clear Failed",
      body: error.localizedDescription,
      attributes: [
        "connectorId": connectorID
      ]
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
    if !command.requiredConnectorIds.isEmpty {
      attributes["connectorIds"] = command.requiredConnectorIds.joined(separator: ", ")
    } else if !command.declaredConnectorIds.isEmpty {
      attributes["connectorIds"] = command.declaredConnectorIds.joined(separator: ", ")
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
    TimelineEntryFactory.warning(
      title: "Plugin Command Failed",
      body: error.localizedDescription,
      attributes: [:]
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
}
