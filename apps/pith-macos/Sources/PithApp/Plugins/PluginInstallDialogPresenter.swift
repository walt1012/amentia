import AppKit
import Foundation

enum PluginInstallDialogPresenter {
  static func confirmInstall(preview: PluginInstallPreview) -> Bool {
    let alert = NSAlert()
    alert.alertStyle = .warning
    alert.messageText = "Add Plugin?"
    alert.informativeText = """
      Plugin: \(preview.displayName) \(preview.version)
      Provenance: Local import
      Author: \(preview.authorName ?? "Unknown")
      Source Folder: \(preview.sourcePath)
      Install Location: \(preview.installPath)
      Starts Enabled: \(preview.defaultEnabled ? "Yes" : "No")
      Access: \(preview.surfaceSummary.summary)
      Needs: \(summaryLine(preview.permissions, empty: "none"))
      Can use: \(summaryLine(preview.capabilities, empty: "none"))

      \(preview.description)
      """
    alert.addButton(withTitle: "Install")
    alert.addButton(withTitle: "Cancel")
    return alert.runModal() == .alertFirstButtonReturn
  }

  static func confirmRemoval(plugin: PluginSummary) -> Bool {
    let alert = NSAlert()
    alert.alertStyle = .warning
    alert.messageText = "Remove Plugin?"
    alert.informativeText = """
      Plugin: \(plugin.displayName) \(plugin.version)
      Provenance: \(plugin.provenance)
      Needs: \(summaryLine(plugin.permissions, empty: "none"))
      Can use: \(summaryLine(plugin.capabilities, empty: "none"))

      Removing this plugin updates the local catalog and can disable related connections, actions, checks, and permissions.
      """
    alert.addButton(withTitle: "Remove")
    alert.addButton(withTitle: "Cancel")
    return alert.runModal() == .alertFirstButtonReturn
  }

  static func repairHint(for error: Error) -> String {
    let message = error.localizedDescription

    if message.contains("\nHint:") || message.contains("\nRepair Hint:") {
      return ""
    }

    if message.contains("does not contain pith-plugin.json") {
      return "Choose a complete plugin folder, or select its setup file directly."
    }

    if message.contains("must be a plugin directory or pith-plugin.json file") {
      return "Choose the plugin directory itself or its setup file."
    }

    if message.contains("is already installed") {
      return "Remove the existing local plugin first, or change the plugin name before installing this copy."
    }

    if message.contains("cannot contain nested pith-plugin.json manifests") {
      return "Remove nested plugin bundles before installing. Install each plugin as its own top-level folder."
    }

    if message.contains("cannot contain symbolic links") {
      return "Replace symlinks with real files or directories so the local plugin bundle is self-contained."
    }

    if message.contains("Select a plugin folder or a pith-plugin.json manifest") {
      return "Point the installer at a plugin directory or its setup file."
    }

    if message.contains("Plugin manifest name") {
      return "Use a stable plugin name without path separators or colons, for example notion-plugin."
    }

    if message.contains("correct format")
      || message.contains("is missing")
    {
      return "Check that the plugin setup file is valid JSON and uses camelCase keys such as displayName and defaultEnabled."
    }

    if message.contains("failed to create plugin install root")
      || message.contains("failed to create")
      || message.contains("failed to copy")
    {
      return "Check local disk permissions and free space, then try installing again."
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
