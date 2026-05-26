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

      if let diagnosticSummary = capability.diagnosticSummary {
        Text(diagnosticSummary)
          .font(.caption2)
          .foregroundColor(capability.diagnosticColor)
          .textSelection(.enabled)
      }

      if let diagnosticDetail = capability.diagnosticDetail {
        Text("Capability blocker: \(diagnosticDetail)")
          .font(.caption2)
          .foregroundColor(.orange)
          .textSelection(.enabled)
      }

      if let metadataSummary = capability.metadataSummary {
        Text("Metadata: \(metadataSummary)")
          .font(.caption2)
          .foregroundColor(.secondary)
          .textSelection(.enabled)
      }
    }
    .padding(.vertical, 4)
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
      .map { "\($0.key)=\($0.value)" }
      .joined(separator: " | ")
  }

  var displaySurface: String {
    switch kind {
    case "command":
      return "Command"
    case "hook":
      return "Hook"
    case "mcp_server":
      return "MCP server"
    default:
      return kind
    }
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

        Button("Manifest") {
          onRevealManifest()
        }
        .font(.caption2)
      }

      Text("\(connector.pluginDisplayName) | \(connector.pluginID)")
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
    if !connector.enabled {
      Button("Enable Plugin") {
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
    let type = authType ?? "none"
    let required = authRequired ? "required" : "optional"
    let scopes = authScopes.isEmpty ? "no scopes" : authScopes.joined(separator: ", ")
    let store = credentialStore ?? "none"
    let credential = credentialLabel ?? "no credential"
    let binding = credentialBinding
    return "Auth: \(type) | \(authStatus) | \(required) | \(scopes) "
      + "| store: \(store) | credential: \(binding) | \(credential)"
  }

  var workflowSummary: String {
    let labels = workflows
      .map { workflow in
        "\(workflow.displayName) / \(workflow.action) / \(workflow.commandCoverageLabel)"
      }
      .joined(separator: ", ")
    return "Workflows: \(labels)"
  }

  private var credentialBinding: String {
    if !credentialPresent {
      return "none"
    }

    return credentialSecretPresent ? "env-bound" : "marker-only"
  }
}
