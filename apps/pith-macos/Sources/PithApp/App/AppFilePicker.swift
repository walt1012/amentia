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
    panel.prompt = "Add Connector"
    panel.message = "Choose a connector folder or its pith-plugin.json setup file."

    guard panel.runModal() == .OK else {
      return nil
    }

    return panel.url
  }
}
