import AppKit
import Foundation

enum PluginCommandInputDialogPresenter {
  static func commandInput(command: PluginCommandSummary) -> String? {
    let alert = NSAlert()
    alert.alertStyle = .informational
    alert.messageText = "Run \(command.title) With Input"
    alert.informativeText = inputPrompt(command)

    let textField = NSTextField(frame: NSRect(x: 0, y: 0, width: 360, height: 24))
    textField.placeholderString = command.requiresPlainInput
      ? "Required command input"
      : "Optional command input"
    alert.accessoryView = textField
    alert.addButton(withTitle: "Run")
    alert.addButton(withTitle: "Cancel")
    alert.window.initialFirstResponder = textField

    guard alert.runModal() == .alertFirstButtonReturn else {
      return nil
    }

    return textField.stringValue.trimmingCharacters(in: .whitespacesAndNewlines)
  }

  private static func inputPrompt(_ command: PluginCommandSummary) -> String {
    let fieldDescription = command.execution?
      .input?
      .fields
      .first(where: { $0.name == "input" })?
      .description

    return fieldDescription ?? "Pass a short text input to this plugin command."
  }
}
