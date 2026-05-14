import SwiftUI

struct PluginCommandRow: View {
  let command: PluginCommandSummary
  let requiredConnectors: [PluginConnectorSummary]
  let canRun: Bool
  let runDisabledReason: String?
  let canAuthorizeConnector: (String) -> Bool
  let onRun: () -> Void
  let onRunWithInput: () -> Void
  let onAuthorizeConnector: (String) -> Void
  let onRevealManifest: () -> Void

  var body: some View {
    VStack(alignment: .leading, spacing: 6) {
      HStack(alignment: .top, spacing: 12) {
        VStack(alignment: .leading, spacing: 2) {
          Text(command.title)
            .font(.caption.weight(.semibold))
          Text("\(command.pluginDisplayName) | \(command.pluginID)")
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
          Button("Manifest") {
            onRevealManifest()
          }
          .font(.caption2)
        }
      }

      Text(command.description)
        .font(.caption2)
        .foregroundColor(.secondary)

      Text("Execution: \(executionLabel)")
        .font(.caption2)
        .foregroundColor(command.runStatus == "ready" ? .secondary : .orange)
        .textSelection(.enabled)

      Text("Run State: \(runStateLabel)")
        .font(.caption2)
        .foregroundColor(command.runStatus == "ready" ? .secondary : .orange)
        .textSelection(.enabled)

      Text("Turn Route: \(command.explicitTurnRoute)")
        .font(.caption2)
        .foregroundColor(.secondary)
        .textSelection(.enabled)

      if let runDisabledReason {
        Text("Run blocker: \(runDisabledReason)")
          .font(.caption2)
          .foregroundColor(.orange)
          .textSelection(.enabled)
      }

      if let repairHint = command.runRepairHint {
        Text("Repair: \(repairHint)")
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

      if let contractLabel {
        Text(contractLabel)
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
        Text("Permissions: \(command.permissions.joined(separator: ", "))")
          .font(.caption2)
          .foregroundColor(.secondary)
          .textSelection(.enabled)
      }
    }
    .padding(.vertical, 4)
  }

  @ViewBuilder
  private var connectorRows: some View {
    if !requiredConnectors.isEmpty {
      ForEach(requiredConnectors) { connector in
        HStack(alignment: .firstTextBaseline, spacing: 8) {
          Text(connectorLabel(connector))
            .font(.caption2)
            .foregroundColor(connector.authStatus == "needsAuth" ? .orange : .secondary)
            .textSelection(.enabled)

          Spacer()

          if connector.authStatus == "needsAuth" {
            Button("Authorize") {
              onAuthorizeConnector(connector.id)
            }
            .font(.caption2)
            .disabled(!canAuthorizeConnector(connector.id))
          }
        }
      }
    } else if !command.requiredConnectorIds.isEmpty {
      Text("Connectors: \(command.requiredConnectorIds.joined(separator: ", "))")
        .font(.caption2)
        .foregroundColor(.secondary)
        .textSelection(.enabled)
    } else if !command.declaredConnectorIds.isEmpty {
      Text("Connectors: \(command.declaredConnectorIds.joined(separator: ", "))")
        .font(.caption2)
        .foregroundColor(.secondary)
        .textSelection(.enabled)
    }
  }

  private var executionLabel: String {
    guard let execution = command.execution else {
      return "missing contract"
    }

    let suffix = execution.supported ? "supported" : "not supported yet"
    return "\(execution.kind) via \(execution.driver) (\(suffix))"
  }

  private var runStateLabel: String {
    if let blocker = command.runBlocker {
      return "\(command.runStatus) | \(blocker)"
    }

    return command.runStatus
  }

  private var approvalLabel: String {
    command.approvalReason ?? "Required before runner launch."
  }

  private var contractLabel: String? {
    guard let execution = command.execution,
          let input = execution.input,
          let output = execution.output
    else {
      return nil
    }

    return "Contract: \(input.envelope) -> \(output.envelope)"
  }

  private var requiredInputLabel: String? {
    guard !command.requiredInputFieldNames.isEmpty else {
      return nil
    }

    return "Required input fields: \(command.requiredInputFieldNames.joined(separator: ", "))"
  }

  private var showsManifestAction: Bool {
    command.runStatus != "ready" || command.execution?.supported == false
  }

  private var canRunDirectly: Bool {
    canRun && !command.requiresPlainInput
  }

  private func connectorLabel(_ connector: PluginConnectorSummary) -> String {
    let secret = connector.credentialSecretPresent ? "env-bound" : "no secret"
    return "Connector: \(connector.displayName) | \(connector.authStatus) | \(secret)"
  }
}

struct PluginHookRow: View {
  let hook: PluginHookSummary

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
        Text("Repair: \(repairHint)")
          .font(.caption2)
          .foregroundColor(.secondary)
          .textSelection(.enabled)
      }

      if !hook.permissions.isEmpty {
        Text("Permissions: \(hook.permissions.joined(separator: ", "))")
          .font(.caption2)
          .foregroundColor(.secondary)
          .textSelection(.enabled)
      }
    }
    .padding(.vertical, 4)
  }
}
