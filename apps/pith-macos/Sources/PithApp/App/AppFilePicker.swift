import AppKit
import Foundation

enum AppFilePicker {
  static func chooseWorkspace() -> URL? {
    let panel = NSOpenPanel()
    panel.canChooseDirectories = true
    panel.canChooseFiles = false
    panel.allowsMultipleSelection = false
    panel.prompt = "Open Project"
    panel.message = "Choose a local project folder for Pith to inspect."

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
    panel.message = "Choose a local connector folder. Advanced users can select its setup file directly."

    guard panel.runModal() == .OK else {
      return nil
    }

    return panel.url
  }
}
