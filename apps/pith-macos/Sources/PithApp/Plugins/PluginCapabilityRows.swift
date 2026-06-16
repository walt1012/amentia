import SwiftUI

struct PluginCapabilityRow: View {
  let capability: PluginCapabilitySummary

  var body: some View {
    VStack(alignment: .leading, spacing: 4) {
      HStack(alignment: .top, spacing: 12) {
        VStack(alignment: .leading, spacing: 2) {
          Text(PluginCapabilityPresenter.title(capability))
            .font(.caption.weight(.semibold))
          Text(capability.pluginDisplayName)
            .font(.caption2)
            .foregroundColor(.secondary)
        }

        Spacer()
      }

      if !capability.permissions.isEmpty {
        Text("Needs: \(PluginPermissionDisplay.summary(capability.permissions))")
          .font(.caption2)
          .foregroundColor(.secondary)
          .textSelection(.enabled)
      }

      if let reviewSummary = PluginCapabilityPresenter.reviewSummary(capability) {
        Text(reviewSummary)
          .font(.caption2)
          .foregroundColor(.secondary)
          .textSelection(.enabled)
      }

      if let diagnosticSummary = PluginCapabilityPresenter.diagnosticSummary(capability) {
        Text(diagnosticSummary)
          .font(.caption2)
          .foregroundColor(PluginCapabilityPresenter.diagnosticColor(capability))
          .textSelection(.enabled)
      }

      if let diagnosticDetail = PluginCapabilityPresenter.diagnosticDetail(capability) {
        Text("Needs attention: \(diagnosticDetail)")
          .font(.caption2)
          .foregroundColor(.orange)
          .textSelection(.enabled)
      }
    }
    .softPanel(
      tone: PluginCapabilityPresenter.diagnosticDetail(capability) == nil ? .neutral : .warning
    )
  }
}

enum PluginCapabilityPresenter {
  static func title(_ capability: PluginCapabilitySummary) -> String {
    if capability.kind == "connector",
       let displayName = cleanMetadataValue(capability.metadata["displayName"])
    {
      return "Connection: \(displayName)"
    }

    PluginCapabilityDisplay.surface(capability.kind)
  }

  static func reviewSummary(_ capability: PluginCapabilitySummary) -> String? {
    switch capability.kind {
    case "connector":
      return connectorReviewSummary(capability)
    case "skill":
      return skillReviewSummary(capability)
    case "mcp_server":
      return mcpServerReviewSummary(capability)
    case "command", "hook":
      return definitionReviewSummary(capability)
    default:
      return nil
    }
  }

  static func diagnosticSummary(_ capability: PluginCapabilitySummary) -> String? {
    if let serverStatus = capability.metadata["serverStatus"] {
      return "MCP server: \(displayStatus(serverStatus))"
    }
    if let definitionStatus = capability.metadata["definitionStatus"] {
      return "\(title(capability)) definition: \(displayStatus(definitionStatus))"
    }
    return nil
  }

  static func diagnosticDetail(_ capability: PluginCapabilitySummary) -> String? {
    if capability.metadata["serverError"] != nil {
      return "Add the missing MCP command in plugin setup."
    }
    if capability.metadata["definitionError"] != nil {
      return "Review this capability definition in plugin setup."
    }
    return nil
  }

  static func diagnosticColor(_ capability: PluginCapabilitySummary) -> Color {
    switch capability.metadata["serverStatus"] ?? capability.metadata["definitionStatus"] {
    case "ready":
      return .secondary
    case nil:
      return .secondary
    default:
      return .orange
    }
  }

  private static func connectorReviewSummary(_ capability: PluginCapabilitySummary) -> String? {
    var parts: [String] = []
    if let service = cleanMetadataValue(capability.metadata["service"]) {
      parts.append("Service: \(PluginStatusDisplay.serviceName(service))")
    }
    if let authRequired = capability.metadata["authRequired"] {
      parts.append(authRequired == "true" ? "authorization required" : "no authorization")
    }
    if let authType = cleanMetadataValue(capability.metadata["authType"]) {
      parts.append("auth: \(displayAuthType(authType))")
    }
    if let authScopes = cleanMetadataValue(capability.metadata["authScopes"]) {
      parts.append("access: \(authScopes)")
    }
    if let credentialStore = cleanMetadataValue(capability.metadata["credentialStore"]) {
      parts.append("stored: \(displayCredentialStore(credentialStore))")
    }
    return parts.isEmpty ? nil : parts.joined(separator: " | ")
  }

  private static func skillReviewSummary(_ capability: PluginCapabilitySummary) -> String? {
    guard let description = cleanMetadataValue(capability.metadata["description"]) else {
      return nil
    }

    return "Guidance: \(description)"
  }

  private static func mcpServerReviewSummary(_ capability: PluginCapabilitySummary) -> String? {
    let transport: String
    if let metadataTransport = cleanMetadataValue(capability.metadata["transport"]) {
      transport = displayTransport(metadataTransport)
    } else {
      transport = "local"
    }
    let commandState = capability.metadata["command"] == nil
      ? "needs a local command"
      : "local command configured"
    return "MCP: \(transport) server, \(commandState)."
  }

  private static func definitionReviewSummary(_ capability: PluginCapabilitySummary) -> String? {
    guard let status = capability.metadata["definitionStatus"] else {
      return nil
    }

    switch status {
    case "ready":
      return "Setup: definition ready."
    case "missing":
      return "Setup: definition missing."
    case "invalid":
      return "Setup: definition needs review."
    default:
      return "Setup: \(displayStatus(status))."
    }
  }

  private static func displayStatus(_ status: String) -> String {
    switch status {
    case "missingCommand":
      return "missing command"
    case "unsupportedTransport":
      return "unsupported transport"
    default:
      return status
    }
  }

  private static func displayAuthType(_ value: String) -> String {
    switch value {
    case "api_key":
      return "API key"
    case "oauth2":
      return "OAuth 2.0"
    default:
      return value.replacingOccurrences(of: "_", with: " ")
    }
  }

  private static func displayCredentialStore(_ value: String) -> String {
    switch value {
    case "local":
      return "local only"
    case "none":
      return "not saved"
    default:
      return value.replacingOccurrences(of: "_", with: " ")
    }
  }

  private static func displayTransport(_ value: String) -> String {
    switch value {
    case "stdio":
      return "local stdio"
    default:
      return value.replacingOccurrences(of: "_", with: " ")
    }
  }

  private static func cleanMetadataValue(_ value: String?) -> String? {
    guard let value = value?.trimmingCharacters(in: .whitespacesAndNewlines),
          !value.isEmpty,
          !value.contains("/"),
          !value.contains("\\")
    else {
      return nil
    }

    return value
  }
}

struct PluginConnectorRow: View {
  let connector: PluginConnectorSummary
  let canEnablePlugin: Bool
  let canAuthorize: Bool
  let canClearCredential: Bool
  let authorizeDisabledReason: String?
  let clearCredentialDisabledReason: String?
  let onAuthorize: () -> Void
  let onClearCredential: () -> Void
  let onEnablePlugin: () -> Void
  let onRevealManifest: () -> Void

  var body: some View {
    VStack(alignment: .leading, spacing: 6) {
      HStack(alignment: .top, spacing: 12) {
        VStack(alignment: .leading, spacing: 2) {
          Text(connector.displayName)
            .font(.caption.weight(.semibold))
          Text(
            "\(PluginStatusDisplay.serviceName(connector.service)) | \(PluginStatusDisplay.connectionStatus(connector.status))"
          )
            .font(.caption2)
            .foregroundColor(statusColor)
        }

        Spacer()

        connectorActions

        Button("Setup") {
          onRevealManifest()
        }
        .font(.caption2)
      }

      Text(connector.pluginDisplayName)
        .font(.caption2)
        .foregroundColor(.secondary)

      Text(connector.authSummary)
        .font(.caption2)
        .foregroundColor(.secondary)
        .textSelection(.enabled)

      if !connector.workflows.isEmpty {
        Text(connector.workflowSummary)
          .font(.caption2)
          .foregroundColor(.secondary)
          .textSelection(.enabled)
      }

      if let actionBlocker {
        Text("Needs attention: \(actionBlocker)")
          .font(.caption2)
          .foregroundColor(.orange)
          .textSelection(.enabled)
      }

      if !connector.permissions.isEmpty {
        Text("Needs: \(PluginPermissionDisplay.summary(connector.permissions))")
          .font(.caption2)
          .foregroundColor(.secondary)
          .textSelection(.enabled)
      }

      if let homepage = connector.homepage {
        Text("Website: \(homepage)")
          .font(.caption2)
          .foregroundColor(.secondary)
          .textSelection(.enabled)
      }
    }
    .softPanel(tone: connector.status == "ready" ? .neutral : .warning)
  }

  @ViewBuilder
  private var connectorActions: some View {
    if !connector.enabled {
      Button("Enable") {
        onEnablePlugin()
      }
      .font(.caption2)
      .disabled(!canEnablePlugin)
    } else if connector.authRequired {
      if connector.credentialPresent {
        Button("Clear") {
          onClearCredential()
        }
        .font(.caption2)
        .disabled(!canClearCredential)
      } else {
        Button("Authorize") {
          onAuthorize()
        }
        .font(.caption2)
        .disabled(!canAuthorize)
      }
    }
  }

  private var actionBlocker: String? {
    guard connector.authRequired else {
      return nil
    }

    if connector.credentialPresent {
      return clearCredentialDisabledReason
    }

    return authorizeDisabledReason
  }

  private var statusColor: Color {
    switch connector.status {
    case "ready":
      return .green
    case "needsAuth":
      return .orange
    default:
      return .secondary
    }
  }
}

private extension PluginConnectorSummary {
  var authSummary: String {
    var parts = [
      "Connection: \(PluginStatusDisplay.authorizationStatus(authStatus, credentialPresent: credentialPresent))"
    ]
    if !authScopes.isEmpty {
      parts.append("access: \(authScopes.joined(separator: ", "))")
    }
    if let credentialLabel {
      parts.append(credentialLabel)
    }
    return parts.joined(separator: " | ")
  }

  var workflowSummary: String {
    let labels = workflows
      .map(\.workflowLabel)
      .joined(separator: ", ")
    return "Can run: \(labels)"
  }

}
