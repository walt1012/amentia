import AppKit
import Foundation

struct PluginConnectorCredentialInput {
  let label: String?
  let tokenOrKey: String?
}

enum PluginConnectorCredentialDialogPresenter {
  static let labelFieldTitle = "Name"
  static let tokenOrKeyFieldTitle = "Token or key"

  static func credentialInput(connector: PluginConnectorSummary) -> PluginConnectorCredentialInput? {
    var initialLabel = connector.credentialLabel ?? defaultCredentialLabel(connector)
    while true {
      guard let input = credentialInput(connector: connector, initialLabel: initialLabel) else {
        return nil
      }
      if input.tokenOrKey != nil {
        return input
      }
      if requiresLocalTokenOrKey(connector) {
        showsMissingTokenOrKeyWarning(connector: connector)
        initialLabel = input.label ?? initialLabel
        continue
      }
      if confirmsMarkerOnlyAuthorization(connector: connector) {
        return input
      }
      initialLabel = input.label ?? initialLabel
    }
  }

  private static func credentialInput(
    connector: PluginConnectorSummary,
    initialLabel: String
  ) -> PluginConnectorCredentialInput? {
    let alert = NSAlert()
    alert.alertStyle = .informational
    alert.messageText = "Authorize \(connector.displayName)"
    alert.informativeText = credentialPrompt(connector)

    let labelField = NSTextField(frame: NSRect(x: 0, y: 0, width: 360, height: 24))
    labelField.placeholderString = "Connection name"
    labelField.stringValue = initialLabel

    let tokenOrKeyField = NSSecureTextField(frame: NSRect(x: 0, y: 0, width: 360, height: 24))
    tokenOrKeyField.placeholderString = tokenOrKeyPlaceholder(connector)

    let stack = NSStackView(views: [
      labeledField(title: labelFieldTitle, field: labelField),
      labeledField(title: tokenOrKeyFieldTitle, field: tokenOrKeyField),
    ])
    stack.orientation = .vertical
    stack.alignment = .leading
    stack.spacing = 8
    alert.accessoryView = stack
    alert.addButton(withTitle: "Authorize")
    alert.addButton(withTitle: "Cancel")
    alert.window.initialFirstResponder = tokenOrKeyField

    guard alert.runModal() == .alertFirstButtonReturn else {
      return nil
    }

    return PluginConnectorCredentialInput(
      label: normalized(labelField.stringValue),
      tokenOrKey: normalized(tokenOrKeyField.stringValue)
    )
  }

  static func credentialPrompt(_ connector: PluginConnectorSummary) -> String {
    let authType = PluginStatusDisplay.authTypeName(connector.authType)
    let service = PluginStatusDisplay.serviceName(connector.service)
    let access = PluginStatusDisplay.accessSummary(connector.authScopes)
      .map { "Access: \($0)." }
      ?? "No extra access declared."
    let storage = storesAuthorization(connector)
      ? "Amentia stores the authorization locally."
      : "Amentia will not save a token."
    var prompt = "\(connector.pluginDisplayName) requests \(authType) access for \(service). "
      + "\(access) \(storage) "
      + "Tokens are passed only to the local plugin runner for each approved run. "
    if requiresLocalTokenOrKey(connector) {
      prompt += "Paste a local token or API key for this connection."
    } else {
      prompt += "Leave the token field empty only when this connection can be approved without a token."
    }
    if let servicePrompt = PluginConnectorServiceGuide.setupPrompt(connector: connector) {
      prompt += servicePrompt
    }
    return prompt
  }

  private static func storesAuthorization(_ connector: PluginConnectorSummary) -> Bool {
    let store = connector.credentialStore?
      .trimmingCharacters(in: .whitespacesAndNewlines)
      .lowercased()
    return store != "none"
  }

  private static func confirmsMarkerOnlyAuthorization(
    connector: PluginConnectorSummary
  ) -> Bool {
    let alert = NSAlert()
    alert.alertStyle = .warning
    alert.messageText = "Authorize \(connector.displayName) Without a Token?"
    alert.informativeText =
      "Amentia can remember that this connection is allowed without saving a token or API key. "
      + "Use this only for connection flows that do not need a local token."
    alert.addButton(withTitle: "Authorize")
    alert.addButton(withTitle: "Back")
    return alert.runModal() == .alertFirstButtonReturn
  }

  private static func showsMissingTokenOrKeyWarning(connector: PluginConnectorSummary) {
    let alert = NSAlert()
    alert.alertStyle = .warning
    alert.messageText = "\(connector.displayName) Needs a Token or Key"
    alert.informativeText = missingTokenOrKeyWarningText(connector)
    alert.addButton(withTitle: "Back")
    alert.runModal()
  }

  static func missingTokenOrKeyWarningText(_ connector: PluginConnectorSummary) -> String {
    if let serviceWarning = PluginConnectorServiceGuide.missingTokenOrKeyWarning(
      connector: connector
    ) {
      return serviceWarning
    }

    return "Paste the local token or API key before authorizing this connection. "
      + "Amentia will keep it local and pass it only to the local plugin runner for each approved run."
  }

  static func defaultCredentialLabel(_ connector: PluginConnectorSummary) -> String {
    PluginConnectorServiceGuide.defaultCredentialLabel(connector: connector)
      ?? "\(connector.displayName) local authorization"
  }

  static func tokenOrKeyPlaceholder(_ connector: PluginConnectorSummary) -> String {
    PluginConnectorServiceGuide.tokenOrKeyPlaceholder(connector: connector)
      ?? "Token or API key, if this connection needs one"
  }

  static func requiresLocalTokenOrKey(_ connector: PluginConnectorSummary) -> Bool {
    guard connector.authRequired else {
      return false
    }

    let authType = connector.authType?
      .trimmingCharacters(in: .whitespacesAndNewlines)
      .lowercased()
      .replacingOccurrences(of: "-", with: "_")

    return authType == "api_key" || authType == "apikey"
  }

  private static func labeledField(title: String, field: NSView) -> NSView {
    let label = NSTextField(labelWithString: title)
    label.font = .systemFont(ofSize: NSFont.smallSystemFontSize)

    let stack = NSStackView(views: [label, field])
    stack.orientation = .vertical
    stack.alignment = .leading
    stack.spacing = 4
    return stack
  }

  private static func normalized(_ value: String) -> String? {
    let trimmed = value.trimmingCharacters(in: .whitespacesAndNewlines)
    return trimmed.isEmpty ? nil : trimmed
  }
}
