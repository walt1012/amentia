import Foundation

extension TimelineEventPresenter {
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
