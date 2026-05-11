import Foundation

enum TimelineEventPresenter {
  static let generatingLocalResponseDetail = "Generating local response..."
  static let pendingTurnCancelledDetail = "Local execution request cancelled."
  static let runningPluginCommandDetail = "Running local plugin command..."
  static let pluginCommandNeedsExecutionContractDetail =
    "Plugin command needs an execution contract before it can run."
  static let pendingPluginCommandCancelledDetail = "Local plugin command cancelled."
  static let cancellingTurnDetail = "Cancelling local execution..."

  static let cancelledResponsePreview = "Cancelled response"
  static let cancellingResponsePreview = "Cancelling response"
  static let cancelledPluginCommandPreview = "Cancelled plugin command"

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

  static func pluginInstallPreviewFailed(error: Error, repairHint: String) -> TimelineEntry {
    let body = repairHint.isEmpty
      ? error.localizedDescription
      : "\(error.localizedDescription)\n\nRepair Hint: \(repairHint)"

    return TimelineEntryFactory.warning(
      title: "Plugin Install Preview Failed",
      body: body,
      attributes: [:]
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
        "pluginInstallPath": preview.installPath,
      ]
    )
  }

  static func pluginInstallFailed(error: Error) -> TimelineEntry {
    TimelineEntryFactory.warning(
      title: "Plugin Install Failed",
      body: error.localizedDescription,
      attributes: [:]
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
}
