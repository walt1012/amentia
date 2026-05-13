import SwiftUI

struct PluginCapabilityRow: View {
  let capability: PluginCapabilitySummary

  var body: some View {
    VStack(alignment: .leading, spacing: 4) {
      HStack(alignment: .top, spacing: 12) {
        VStack(alignment: .leading, spacing: 2) {
          Text("\(capability.kind):\(capability.identifier)")
            .font(.caption.weight(.semibold))
          Text("\(capability.pluginDisplayName) | \(capability.pluginID)")
            .font(.caption2)
            .foregroundColor(.secondary)
        }

        Spacer()
      }

      if !capability.permissions.isEmpty {
        Text("Permissions: \(capability.permissions.joined(separator: ", "))")
          .font(.caption2)
          .foregroundColor(.secondary)
          .textSelection(.enabled)
      }

      if !capability.metadata.isEmpty {
        Text("Metadata: \(capability.metadataSummary)")
          .font(.caption2)
          .foregroundColor(.secondary)
          .textSelection(.enabled)
      }
    }
    .padding(.vertical, 4)
  }
}

private extension PluginCapabilitySummary {
  var metadataSummary: String {
    metadata
      .sorted(by: { $0.key < $1.key })
      .map { "\($0.key)=\($0.value)" }
      .joined(separator: " | ")
  }
}

struct PluginConnectorRow: View {
  let connector: PluginConnectorSummary
  let canAuthorize: Bool
  let canClearCredential: Bool
  let authorizeDisabledReason: String?
  let clearCredentialDisabledReason: String?
  let onAuthorize: () -> Void
  let onClearCredential: () -> Void

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
      }

      Text("\(connector.pluginDisplayName) | \(connector.pluginID)")
        .font(.caption2)
        .foregroundColor(.secondary)

      Text(connector.authSummary)
        .font(.caption2)
        .foregroundColor(.secondary)
        .textSelection(.enabled)

      if let actionBlocker {
        Text("Connector blocker: \(actionBlocker)")
          .font(.caption2)
          .foregroundColor(.orange)
          .textSelection(.enabled)
      }

      if !connector.permissions.isEmpty {
        Text("Permissions: \(connector.permissions.joined(separator: ", "))")
          .font(.caption2)
          .foregroundColor(.secondary)
          .textSelection(.enabled)
      }

      if let homepage = connector.homepage {
        Text("Homepage: \(homepage)")
          .font(.caption2)
          .foregroundColor(.secondary)
          .textSelection(.enabled)
      }
    }
    .padding(.vertical, 4)
  }

  @ViewBuilder
  private var connectorActions: some View {
    if connector.authRequired {
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
    let type = authType ?? "none"
    let required = authRequired ? "required" : "optional"
    let scopes = authScopes.isEmpty ? "no scopes" : authScopes.joined(separator: ", ")
    let store = credentialStore ?? "none"
    let provider = credentialProvider ?? "none"
    let handle = credentialHandle ?? "none"
    let credential = credentialLabel ?? "no credential"
    let secret = credentialSecretPresent ? "env-bound" : "none"
    return "Auth: \(type) | \(authStatus) | \(required) | \(scopes) "
      + "| store: \(store) | provider: \(provider) | handle: \(handle) "
      + "| secret: \(secret) | \(credential)"
  }
}
