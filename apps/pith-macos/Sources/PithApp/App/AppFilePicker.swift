import AppKit
import Foundation

enum AppFilePicker {
  static func chooseWorkspace() -> URL? {
    let panel = NSOpenPanel()
    panel.canChooseDirectories = true
    panel.canChooseFiles = false
    panel.allowsMultipleSelection = false
    panel.prompt = "Open Workspace"
    panel.message = "Choose a local workspace for Pith to inspect."

    guard panel.runModal() == .OK else {
      return nil
    }

    return panel.url
  }

  static func choosePluginSource() -> URL? {
    let panel = NSOpenPanel()
    panel.canChooseDirectories = true
    panel.canChooseFiles = true
    panel.allowsMultipleSelection = false
    panel.prompt = "Install Plugin"
    panel.message = "Choose a plugin folder or a pith-plugin.json manifest."

    guard panel.runModal() == .OK else {
      return nil
    }

    return panel.url
  }
}
