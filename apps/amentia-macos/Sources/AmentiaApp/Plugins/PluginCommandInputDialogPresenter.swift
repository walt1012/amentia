import AppKit
import Foundation

enum PluginCommandInputDialogPresenter {
  static func commandInput(
    command: PluginCommandSummary,
    initialValue: String? = nil,
    informativeText: String? = nil
  ) -> String? {
    let alert = NSAlert()
    alert.alertStyle = .informational
    alert.messageText = "Run \(command.title) With Input"
    alert.informativeText = inputPrompt(command, override: informativeText)

    let textView = NSTextView(frame: NSRect(x: 0, y: 0, width: 440, height: 150))
    textView.isRichText = false
    textView.isAutomaticQuoteSubstitutionEnabled = false
    textView.isAutomaticDashSubstitutionEnabled = false
    textView.font = NSFont.monospacedSystemFont(ofSize: 12, weight: .regular)
    textView.string = initialValue ?? ""

    let scrollView = NSScrollView(frame: NSRect(x: 0, y: 0, width: 440, height: 150))
    scrollView.borderType = .bezelBorder
    scrollView.hasVerticalScroller = true
    scrollView.documentView = textView
    alert.accessoryView = scrollView
    alert.addButton(withTitle: "Run")
    alert.addButton(withTitle: "Cancel")
    alert.window.initialFirstResponder = textView

    guard alert.runModal() == .alertFirstButtonReturn else {
      return nil
    }

    return textView.string.trimmingCharacters(in: .whitespacesAndNewlines)
  }

  static func inputPrompt(
    _ command: PluginCommandSummary,
    override: String?
  ) -> String {
    let fieldDescription = command.execution?
      .input?
      .fields
      .first(where: { $0.name == "input" })?
      .description

    var prompt = override
      ?? fieldDescription
      ?? "Pass a short text input to this action."
    if let serviceAppendix = PluginConnectorServiceGuide.commandInputAppendix(command: command) {
      prompt += "\n\n\(serviceAppendix)"
    }
    return prompt
  }
}
