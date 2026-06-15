import SwiftUI

struct PluginCapabilityRow: View {
  let capability: PluginCapabilitySummary

  var body: some View {
    VStack(alignment: .leading, spacing: 4) {
      HStack(alignment: .top, spacing: 12) {
        VStack(alignment: .leading, spacing: 2) {
          Text(capability.displayTitle)
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

      if let diagnosticSummary = capability.diagnosticSummary {
        Text(diagnosticSummary)
          .font(.caption2)
          .foregroundColor(capability.diagnosticColor)
          .textSelection(.enabled)
      }

      if let diagnosticDetail = capability.diagnosticDetail {
        Text("Needs attention: \(diagnosticDetail)")
          .font(.caption2)
          .foregroundColor(.orange)
          .textSelection(.enabled)
      }

      if let metadataSummary = capability.metadataSummary {
        Text(metadataSummary)
          .font(.caption2)
          .foregroundColor(.secondary)
          .textSelection(.enabled)
      }
    }
    .softPanel(tone: capability.diagnosticDetail == nil ? .neutral : .warning)
  }
}

private extension PluginCapabilitySummary {
  var diagnosticSummary: String? {
    if let serverStatus = metadata["serverStatus"] {
      return "MCP server: \(displayStatus(serverStatus))"
    }
    if let definitionStatus = metadata["definitionStatus"] {
      return "\(displaySurface) definition: \(displayStatus(definitionStatus))"
    }
    return nil
  }

  var diagnosticDetail: String? {
    metadata["serverError"] ?? metadata["definitionError"]
  }

  var diagnosticColor: Color {
    switch metadata["serverStatus"] ?? metadata["definitionStatus"] {
    case "ready":
      return .secondary
    case nil:
      return .secondary
    default:
      return .orange
    }
  }

  var displayTitle: String {
    "\(displaySurface): \(identifier)"
  }

  var metadataSummary: String? {
    let diagnosticKeys = Set([
      "definitionError",
      "definitionStatus",
      "serverError",
      "serverStatus"
    ])
    let visibleMetadata = metadata
      .filter { !diagnosticKeys.contains($0.key) }
      .sorted(by: { $0.key < $1.key })
    guard !visibleMetadata.isEmpty else {
      return nil
    }

    return visibleMetadata
      .map { "\($0.key): \($0.value)" }
      .joined(separator: " | ")
  }

  var displaySurface: String {
    PluginCapabilityDisplay.surface(kind)
  }

  func displayStatus(_ status: String) -> String {
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
          Text("\(connector.service) | \(connector.status)")
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
      "Connection: \(displayStatus(authStatus))"
    ]
    if authRequired {
      parts.append("sign-in required")
    }
    if !authScopes.isEmpty {
      parts.append("access: \(authScopes.joined(separator: ", "))")
    }
    if credentialPresent {
      parts.append("authorization saved")
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

  private func displayStatus(_ status: String) -> String {
    switch status {
    case "ready":
      return "ready"
    case "needsAuth":
      return "needs sign in"
    default:
      return status
    }
  }
}
