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

        Button("Reveal Manifest") {
          onRevealManifest()
        }
        .buttonStyle(.bordered)
      }

      if plugin.permissions.isEmpty {
        Text("No extra runtime permissions declared.")
          .font(.caption2)
          .foregroundColor(.secondary)
      } else {
        Text(plugin.permissions.joined(separator: ", "))
          .font(.caption2)
          .foregroundColor(.secondary)
          .textSelection(.enabled)
      }
    }
    .padding(.vertical, 4)
  }
}

struct InvalidPluginRow: View {
  let plugin: PluginSummary
  let canRemove: Bool
  let onRevealManifest: () -> Void
  let onRemove: () -> Void

  var body: some View {
    VStack(alignment: .leading, spacing: 6) {
      HStack(alignment: .top, spacing: 12) {
        VStack(alignment: .leading, spacing: 2) {
          Text(plugin.displayName)
            .font(.caption.weight(.semibold))
          Text(plugin.manifestPath)
            .font(.caption2)
            .foregroundColor(.secondary)
            .textSelection(.enabled)
        }

        Spacer()

        Button("Reveal Manifest") {
          onRevealManifest()
        }
        .buttonStyle(.bordered)

        if plugin.provenance == "local" {
          Button("Remove Local Plugin") {
            onRemove()
          }
          .buttonStyle(.bordered)
          .disabled(!canRemove)
        }
      }

      Text(plugin.validationError ?? "Plugin manifest did not pass runtime validation.")
        .font(.caption2)
        .foregroundColor(.orange)
        .textSelection(.enabled)

      if let validationHint = plugin.validationHint {
        Text("Repair: \(validationHint)")
          .font(.caption2)
          .foregroundColor(.secondary)
          .textSelection(.enabled)
      }
    }
    .padding(.vertical, 4)
  }
}
