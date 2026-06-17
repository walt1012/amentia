import SwiftUI

struct PluginPermissionRow: View {
  let plugin: PluginSummary
  let onRevealManifest: () -> Void

  var body: some View {
    VStack(alignment: .leading, spacing: 6) {
      HStack(alignment: .top, spacing: 12) {
        VStack(alignment: .leading, spacing: 2) {
          Text(plugin.displayName)
            .font(.caption.weight(.semibold))
          Text(plugin.enabled ? "Enabled" : "Disabled")
            .font(.caption2)
            .foregroundColor(.secondary)
        }

        Spacer()

        Button("Show Setup") {
          onRevealManifest()
        }
        .buttonStyle(.bordered)
      }

      if plugin.permissions.isEmpty {
        Text("No extra local permissions requested.")
          .font(.caption2)
          .foregroundColor(.secondary)
      } else {
        Text(PluginPermissionDisplay.summary(plugin.permissions))
          .font(.caption2)
          .foregroundColor(.secondary)
          .textSelection(.enabled)
      }
    }
    .softPanel()
  }
}

struct InvalidPluginRow: View {
  let plugin: PluginSummary
  let canRemove: Bool
  let canRefresh: Bool
  let refreshDisabledReason: String?
  let onRevealManifest: () -> Void
  let onRefresh: () -> Void
  let onRemove: () -> Void

  var body: some View {
    VStack(alignment: .leading, spacing: 6) {
      HStack(alignment: .top, spacing: 12) {
        VStack(alignment: .leading, spacing: 2) {
          Text(plugin.displayName)
            .font(.caption.weight(.semibold))
          Text("Plugin setup needs attention.")
            .font(.caption2)
            .foregroundColor(.secondary)
        }

        Spacer()

        Button("Show Setup") {
          onRevealManifest()
        }
        .buttonStyle(.bordered)

        Button("Refresh") {
          onRefresh()
        }
        .buttonStyle(.bordered)
        .disabled(!canRefresh)

        if plugin.provenance == "local" {
          Button("Remove") {
            onRemove()
          }
          .buttonStyle(.bordered)
          .disabled(!canRemove)
        }
      }

      Text(PluginStatusDisplay.validationDetail(plugin))
        .font(.caption2)
        .foregroundColor(.orange)
        .textSelection(.enabled)

      if let refreshDisabledReason {
        Text("Refresh blocked: \(refreshDisabledReason)")
          .font(.caption2)
          .foregroundColor(.secondary)
          .textSelection(.enabled)
      }
    }
    .softPanel(tone: .warning)
  }
}
