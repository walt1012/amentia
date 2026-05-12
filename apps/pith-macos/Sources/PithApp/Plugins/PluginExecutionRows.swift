import SwiftUI

struct PluginCommandRow: View {
  let command: PluginCommandSummary
  let canRun: Bool
  let onRun: () -> Void

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
        .disabled(!canRun)
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

      if let contractLabel {
        Text(contractLabel)
          .font(.caption2)
          .foregroundColor(.secondary)
          .textSelection(.enabled)
      }

      if let memorySummary = command.memorySummary {
        Text(memorySummary)
          .font(.caption2)
          .foregroundColor(.secondary)
          .textSelection(.enabled)
      }

      if !command.requiredConnectorIds.isEmpty {
        Text("Connectors: \(command.requiredConnectorIds.joined(separator: ", "))")
          .font(.caption2)
          .foregroundColor(.secondary)
          .textSelection(.enabled)
      }

      if !command.permissions.isEmpty {
        Text("Permissions: \(command.permissions.joined(separator: ", "))")
          .font(.caption2)
          .foregroundColor(.secondary)
          .textSelection(.enabled)
      }
    }
    .padding(.vertical, 4)
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

  private var contractLabel: String? {
    guard let execution = command.execution,
          let input = execution.input,
          let output = execution.output
    else {
      return nil
    }

    return "Contract: \(input.envelope) -> \(output.envelope)"
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
            .foregroundColor(.secondary)
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
