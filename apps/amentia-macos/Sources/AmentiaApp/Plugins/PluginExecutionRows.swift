import SwiftUI

struct PluginCommandRow: View {
  let command: PluginCommandSummary
  let connectors: [PluginConnectorSummary]
  let canRun: Bool
  let canRefresh: Bool
  let runDisabledReason: String?
  let canEnablePlugin: (String) -> Bool
  let canAuthorizeConnector: (String) -> Bool
  let onRun: () -> Void
  let onRunWithInput: () -> Void
  let onAuthorizeConnector: (String) -> Void
  let onEnablePlugin: (String) -> Void
  let onRevealSource: () -> Void
  let onRefresh: () -> Void

  var body: some View {
    VStack(alignment: .leading, spacing: 6) {
      HStack(alignment: .top, spacing: 12) {
        VStack(alignment: .leading, spacing: 2) {
          Text(command.title)
            .font(.caption.weight(.semibold))
          Text(command.pluginDisplayName)
            .font(.caption2)
            .foregroundColor(.secondary)
        }

        Spacer()

        Button("Run") {
          onRun()
        }
        .buttonStyle(.bordered)
        .disabled(!canRunDirectly)

        if command.acceptsPlainInput {
          Button("Run with Input") {
            onRunWithInput()
          }
          .font(.caption2)
          .disabled(!canRun)
        }

        if showsManifestAction {
          Button("Setup") {
            onRevealSource()
          }
          .font(.caption2)

          Button("Refresh") {
            onRefresh()
          }
          .font(.caption2)
          .disabled(!canRefresh)
        }
      }

      Text(command.description)
        .font(.caption2)
        .foregroundColor(.secondary)

      Text("Action setup: \(executionLabel)")
        .font(.caption2)
        .foregroundColor(command.runStatus == "ready" ? .secondary : .orange)
        .textSelection(.enabled)

      Text("Status: \(runStateLabel)")
        .font(.caption2)
        .foregroundColor(command.runStatus == "ready" ? .secondary : .orange)
        .textSelection(.enabled)

      if let runDisabledReason {
        Text("Needs attention: \(runDisabledReason)")
          .font(.caption2)
          .foregroundColor(.orange)
          .textSelection(.enabled)
      }

      if let repairHint = command.runRepairHint {
        Text("Fix: \(repairHint)")
          .font(.caption2)
          .foregroundColor(.secondary)
          .textSelection(.enabled)
      }

      if command.requiresPlainInput {
        Text("Input required: use Run with Input.")
          .font(.caption2)
          .foregroundColor(.secondary)
          .textSelection(.enabled)
      }

      if command.approvalRequired {
        Text("Approval: \(approvalLabel)")
          .font(.caption2)
          .foregroundColor(.secondary)
          .textSelection(.enabled)
      }

      if let requiredInputLabel {
        Text(requiredInputLabel)
          .font(.caption2)
          .foregroundColor(command.unsupportedRequiredInputFieldNames.isEmpty ? .secondary : .orange)
          .textSelection(.enabled)
      }

      if let memorySummary = command.memorySummary {
        Text(memorySummary)
          .font(.caption2)
          .foregroundColor(.secondary)
          .textSelection(.enabled)
      }

      connectorRows

      if !command.permissions.isEmpty {
        Text("Needs: \(PluginPermissionDisplay.summary(command.permissions))")
          .font(.caption2)
          .foregroundColor(.secondary)
          .textSelection(.enabled)
      }
    }
    .softPanel(tone: command.runStatus == "ready" ? .neutral : .warning)
  }

  @ViewBuilder
  private var connectorRows: some View {
    if !connectors.isEmpty {
      ForEach(connectors) { connector in
        HStack(alignment: .firstTextBaseline, spacing: 8) {
          Text(connectorLabel(connector))
            .font(.caption2)
            .foregroundColor(connectorColor(connector))
            .textSelection(.enabled)

          Spacer()

          if !connector.enabled {
            Button("Enable") {
              onEnablePlugin(connector.pluginID)
            }
            .font(.caption2)
            .disabled(!canEnablePlugin(connector.pluginID))
          } else if connector.authStatus == "needsAuth" {
            Button("Authorize") {
              onAuthorizeConnector(connector.id)
            }
            .font(.caption2)
            .disabled(!canAuthorizeConnector(connector.id))
          }
        }
      }
    }

    if !missingConnectorIds.isEmpty {
      HStack(alignment: .firstTextBaseline, spacing: 8) {
        Text(PluginStatusDisplay.missingConnectionSummary(count: missingConnectorIds.count))
          .font(.caption2)
          .foregroundColor(.orange)

        Spacer()

        Button("Setup") {
          onRevealSource()
        }
        .font(.caption2)
      }
    }
  }

  private var executionLabel: String {
    PluginStatusDisplay.executionSummary(command.execution)
  }

  private var runStateLabel: String {
    if let blocker = command.runBlocker {
      return "\(PluginStatusDisplay.commandStatus(command.runStatus)) | \(blocker)"
    }

    return PluginStatusDisplay.commandStatus(command.runStatus)
  }

  private var approvalLabel: String {
    command.approvalReason ?? "Required before this action runs."
  }

  private var requiredInputLabel: String? {
    guard !command.requiredInputFieldNames.isEmpty else {
      return nil
    }

    let labels = command.requiredInputFieldNames.map(PluginStatusDisplay.inputFieldLabel)
    return "Input: \(labels.joined(separator: ", "))"
  }

  private var showsManifestAction: Bool {
    command.runStatus != "ready" || command.execution?.supported == false
  }

  private var canRunDirectly: Bool {
    canRun && !command.requiresPlainInput
  }

  private func connectorLabel(_ connector: PluginConnectorSummary) -> String {
    let status = PluginStatusDisplay.connectionStatus(connector.status)
    let authorization = PluginStatusDisplay.authorizationStatus(
      connector.authStatus,
      credentialPresent: connector.credentialPresent
    )
    return "Connection: \(connector.displayName) | \(status) | \(authorization)"
  }

  private var missingConnectorIds: [String] {
    command.visibleConnectorIds.filter { connectorID in
      !connectors.contains { $0.id == connectorID }
    }
  }

  private func connectorColor(_ connector: PluginConnectorSummary) -> Color {
    switch connector.status {
    case "ready":
      return .secondary
    case "needsAuth", "disabled":
      return .orange
    default:
      return .secondary
    }
  }
}

struct PluginHookRow: View {
  let hook: PluginHookSummary
  let canRefresh: Bool
  let onRevealSource: () -> Void
  let onRefresh: () -> Void

  var body: some View {
    VStack(alignment: .leading, spacing: 6) {
      HStack(alignment: .top, spacing: 12) {
        VStack(alignment: .leading, spacing: 2) {
          Text(hook.title)
            .font(.caption.weight(.semibold))
          Text("\(hook.pluginDisplayName) | \(hook.event)")
            .font(.caption2)
            .foregroundColor(hook.status == "ready" ? .secondary : .orange)
        }

        Spacer()

        if hook.status != "ready" {
          Button("Source") {
            onRevealSource()
          }
          .font(.caption2)

          Button("Refresh") {
            onRefresh()
          }
          .font(.caption2)
          .disabled(!canRefresh)
        }
      }

      Text(hook.description)
        .font(.caption2)
        .foregroundColor(.secondary)

      if let memorySummary = hook.memorySummary {
        Text(memorySummary)
          .font(.caption2)
          .foregroundColor(.secondary)
          .textSelection(.enabled)
      }

      if let runBlocker = hook.runBlocker {
        Text(runBlocker)
          .font(.caption2)
          .foregroundColor(.orange)
          .textSelection(.enabled)
      }

      if let repairHint = hook.runRepairHint {
        Text("Fix: \(repairHint)")
          .font(.caption2)
          .foregroundColor(.secondary)
          .textSelection(.enabled)
      }

      if !hook.permissions.isEmpty {
        Text("Needs: \(PluginPermissionDisplay.summary(hook.permissions))")
          .font(.caption2)
          .foregroundColor(.secondary)
          .textSelection(.enabled)
      }
    }
    .softPanel(tone: hook.status == "ready" ? .neutral : .warning)
  }

}
