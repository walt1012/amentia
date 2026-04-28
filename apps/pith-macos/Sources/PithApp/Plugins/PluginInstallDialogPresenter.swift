import AppKit
import Foundation

enum PluginInstallDialogPresenter {
  static func confirmInstall(preview: PluginInstallPreview) -> Bool {
    let alert = NSAlert()
    alert.alertStyle = .warning
    alert.messageText = "Install Plugin?"
    alert.informativeText = """
      Plugin: \(preview.displayName) \(preview.version)
      Provenance: Local import
      Author: \(preview.authorName ?? "Unknown")
      Source: \(preview.sourcePath)
      Manifest: \(preview.manifestPath)
      Install Path: \(preview.installPath)
      Default Enabled: \(preview.defaultEnabled ? "Yes" : "No")
      Permissions: \(summaryLine(preview.permissions, empty: "none"))
      Capabilities: \(summaryLine(preview.capabilities, empty: "none"))

      \(preview.description)
      """
    alert.addButton(withTitle: "Install")
    alert.addButton(withTitle: "Cancel")
    return alert.runModal() == .alertFirstButtonReturn
  }

  static func confirmRemoval(plugin: PluginSummary) -> Bool {
    let alert = NSAlert()
    alert.alertStyle = .warning
    alert.messageText = "Remove Local Plugin?"
    alert.informativeText = """
      Plugin: \(plugin.displayName) \(plugin.version)
      Provenance: \(plugin.provenance)
      Manifest: \(plugin.manifestPath)
      Permissions: \(summaryLine(plugin.permissions, empty: "none"))
      Capabilities: \(summaryLine(plugin.capabilities, empty: "none"))

      Removing this plugin updates the local catalog and can disable related commands, hooks, and permissions.
      """
    alert.addButton(withTitle: "Remove")
    alert.addButton(withTitle: "Cancel")
    return alert.runModal() == .alertFirstButtonReturn
  }

  static func repairHint(for error: Error) -> String {
    let message = error.localizedDescription

    if message.contains("does not contain pith-plugin.json") {
      return "Choose a plugin folder that contains pith-plugin.json, or select the manifest file directly."
    }

    if message.contains("Select a plugin folder or a pith-plugin.json manifest") {
      return "Point the installer at a plugin directory or the manifest file itself."
    }

    if message.contains("Plugin manifest name") {
      return "Use a stable plugin name without path separators or colons, for example notion-connector."
    }

    if message.contains("correct format")
      || message.contains("is missing")
    {
      return "Check that pith-plugin.json is valid JSON and uses camelCase keys such as displayName and defaultEnabled."
    }

    return ""
  }

  private static func summaryLine(_ values: [String], empty: String) -> String {
    if values.isEmpty {
      return empty
    }

    return values.joined(separator: ", ")
  }
}
