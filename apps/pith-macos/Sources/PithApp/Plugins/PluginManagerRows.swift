import SwiftUI

struct PluginRow: View {
  let plugin: PluginSummary
  let canEdit: Bool
  let canRemove: Bool
  let onSetEnabled: (Bool) -> Void
  let onRemove: () -> Void

  var body: some View {
    VStack(alignment: .leading, spacing: 6) {
      HStack(alignment: .top, spacing: 12) {
        VStack(alignment: .leading, spacing: 2) {
          Text(plugin.displayName)
            .font(.subheadline.weight(.semibold))
          Text("\(plugin.version) | \(plugin.provenance) | \(displayStatus)")
            .font(.caption2)
            .foregroundColor(.secondary)
        }

        Spacer()

        Toggle(
          "",
          isOn: Binding(
            get: { plugin.enabled },
            set: onSetEnabled
          )
        )
        .labelsHidden()
        .disabled(!canEdit)
      }

      if canRemove {
        Button("Remove") {
          onRemove()
        }
        .buttonStyle(.bordered)
      }

      Text(plugin.description)
        .font(.caption)
        .foregroundColor(.secondary)

      if !plugin.capabilities.isEmpty {
        Text("Can use: \(plugin.capabilities.joined(separator: ", "))")
          .font(.caption2)
          .foregroundColor(.secondary)
          .textSelection(.enabled)
      }

      if !plugin.permissions.isEmpty {
        Text("Needs: \(plugin.permissions.joined(separator: ", "))")
          .font(.caption2)
          .foregroundColor(.secondary)
          .textSelection(.enabled)
      }

      if let validationError = plugin.validationError {
        Text(validationError)
          .font(.caption2)
          .foregroundColor(.orange)
          .textSelection(.enabled)
      }

      if let validationHint = plugin.validationHint {
        Text("Fix: \(validationHint)")
          .font(.caption2)
          .foregroundColor(.secondary)
          .textSelection(.enabled)
      }
    }
    .softPanel(tone: plugin.status == "ready" ? .neutral : .warning)
  }

  private var displayStatus: String {
    switch plugin.status {
    case "ready":
      return "Ready"
    default:
      return plugin.status
    }
  }
}
