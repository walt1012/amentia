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
          Text("\(plugin.version) | \(plugin.sourceLabel) | \(displayStatus)")
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
        Text("Capabilities: \(plugin.capabilitySummary)")
          .font(.caption2)
          .foregroundColor(.secondary)
          .textSelection(.enabled)
      }

      if !plugin.permissions.isEmpty {
        Text("Needs: \(plugin.permissionSummary)")
          .font(.caption2)
          .foregroundColor(.secondary)
          .textSelection(.enabled)
      }

      if plugin.validationError != nil || plugin.validationHint != nil {
        Text(PluginStatusDisplay.validationDetail(plugin))
          .font(.caption2)
          .foregroundColor(.orange)
          .textSelection(.enabled)
      }
    }
    .softPanel(tone: plugin.status == "ready" ? .neutral : .warning)
  }

  private var displayStatus: String {
    PluginStatusDisplay.pluginStatus(plugin.status).capitalized
  }
}
