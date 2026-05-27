import AppKit
import Foundation

struct PluginConnectorCredentialInput {
  let label: String?
  let secret: String?
}

enum PluginConnectorCredentialDialogPresenter {
  static func credentialInput(connector: PluginConnectorSummary) -> PluginConnectorCredentialInput? {
    var initialLabel = connector.credentialLabel ?? defaultCredentialLabel(connector)
    while true {
      guard let input = credentialInput(connector: connector, initialLabel: initialLabel) else {
        return nil
      }
      if input.secret != nil {
        return input
      }
      if requiresLocalSecret(connector) {
        showsMissingSecretWarning(connector: connector)
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
    labelField.placeholderString = "Credential label"
    labelField.stringValue = initialLabel

    let secretField = NSSecureTextField(frame: NSRect(x: 0, y: 0, width: 360, height: 24))
    secretField.placeholderString = secretPlaceholder(connector)

    let stack = NSStackView(views: [
      labeledField(title: "Label", field: labelField),
      labeledField(title: "Secret", field: secretField),
    ])
    stack.orientation = .vertical
    stack.alignment = .leading
    stack.spacing = 8
    alert.accessoryView = stack
    alert.addButton(withTitle: "Authorize")
    alert.addButton(withTitle: "Cancel")
    alert.window.initialFirstResponder = secretField

    guard alert.runModal() == .alertFirstButtonReturn else {
      return nil
    }

    return PluginConnectorCredentialInput(
      label: normalized(labelField.stringValue),
      secret: normalized(secretField.stringValue)
    )
  }

  static func credentialPrompt(_ connector: PluginConnectorSummary) -> String {
    let authType = displayAuthType(connector.authType)
    let store = connector.credentialStore ?? "local"
    let scopes = connector.authScopes.isEmpty
      ? "No declared scopes."
      : "Scopes: \(connector.authScopes.joined(separator: ", "))."
    var prompt = "\(connector.pluginDisplayName) requests \(authType) access for \(connector.service). "
      + "\(scopes) Credential store: \(store). "
      + "Secrets are passed to plugin runners through per-run environment bindings. "
    if requiresLocalSecret(connector) {
      prompt += "A local token or API key is required for this connector."
    } else {
      prompt += "Leave the secret empty only when this connector uses marker-only authorization."
    }
    if isNotion(connector) {
      prompt += "\n\nNotion setup: create a local Notion integration, copy its internal integration token, and share the target parent page with that integration. Pith does not claim OAuth yet; this token stays in local connector state."
    }
    return prompt
  }

  private static func displayAuthType(_ authType: String?) -> String {
    switch authType {
    case "api_key":
      return "API key"
    case "oauth2":
      return "OAuth 2.0"
    case let value? where !value.isEmpty:
      return value
    default:
      return "local credential"
    }
  }

  private static func confirmsMarkerOnlyAuthorization(
    connector: PluginConnectorSummary
  ) -> Bool {
    let alert = NSAlert()
    alert.alertStyle = .warning
    alert.messageText = "Authorize \(connector.displayName) Without a Secret?"
    alert.informativeText =
      "Pith can store an authorization marker without a token or API key. "
      + "Use this only for connector flows that do not need a local secret."
    alert.addButton(withTitle: "Authorize Marker")
    alert.addButton(withTitle: "Back")
    return alert.runModal() == .alertFirstButtonReturn
  }

  private static func showsMissingSecretWarning(connector: PluginConnectorSummary) {
    let alert = NSAlert()
    alert.alertStyle = .warning
    alert.messageText = "\(connector.displayName) Requires a Secret"
    alert.informativeText =
      "Paste the local token or API key before authorizing this connector. "
      + "Pith will keep it local and pass it to plugin runners through per-run environment bindings."
    alert.addButton(withTitle: "Back")
    alert.runModal()
  }

  private static func defaultCredentialLabel(_ connector: PluginConnectorSummary) -> String {
    isNotion(connector)
      ? "Local Notion integration token"
      : "\(connector.displayName) authorization marker"
  }

  private static func secretPlaceholder(_ connector: PluginConnectorSummary) -> String {
    if isNotion(connector) {
      return "Paste the Notion internal integration token"
    }
    return "Token or API key, or leave blank for marker-only auth"
  }

  private static func isNotion(_ connector: PluginConnectorSummary) -> Bool {
    connector.service.lowercased() == "notion"
  }

  static func requiresLocalSecret(_ connector: PluginConnectorSummary) -> Bool {
    connector.authRequired && connector.authType == "api_key"
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
