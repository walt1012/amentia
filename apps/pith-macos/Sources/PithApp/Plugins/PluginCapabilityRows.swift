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
    PluginCapabilityDisplay.surface(capability.kind)
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
