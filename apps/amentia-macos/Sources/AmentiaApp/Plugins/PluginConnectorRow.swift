import SwiftUI

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
      if connector.needsAuthorization {
        Button("Authorize") {
          onAuthorize()
        }
        .font(.caption2)
        .disabled(!canAuthorize)
      } else if connector.credentialPresent {
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

    if connector.needsAuthorization {
      return authorizeDisabledReason
    }

    if connector.credentialPresent {
      return clearCredentialDisabledReason
    }

    return authorizeDisabledReason
  }

  private var statusColor: Color {
    if connector.needsAuthorization {
      return .orange
    }

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
  var needsAuthorization: Bool {
    authSummaryStatus == "needs sign in"
  }

  var authSummary: String {
    let authorization = authSummaryStatus
    var parts = [
      "Connection: \(authorization)"
    ]
    if let access = PluginStatusDisplay.accessSummary(authScopes) {
      parts.append("access: \(access)")
    }
    if let credentialLabel {
      parts.append(credentialLabel)
    }
    return parts.joined(separator: " | ")
  }

  private var authSummaryStatus: String {
    PluginStatusDisplay.authorizationStatus(
      authStatus,
      authRequired: authRequired,
      credentialPresent: credentialPresent,
      credentialSecretPresent: credentialSecretPresent
    )
  }

  var workflowSummary: String {
    let labels = workflows
      .map(\.workflowLabel)
      .joined(separator: ", ")
    return "Can run: \(labels)"
  }
}
