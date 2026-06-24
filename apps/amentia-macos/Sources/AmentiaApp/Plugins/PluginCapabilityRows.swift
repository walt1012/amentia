import SwiftUI

struct PluginCapabilityRow: View {
  let capability: PluginCapabilitySummary

  var body: some View {
    VStack(alignment: .leading, spacing: 4) {
      HStack(alignment: .top, spacing: 12) {
        VStack(alignment: .leading, spacing: 2) {
          Text(PluginCapabilityPresenter.title(capability))
            .font(.caption.weight(.semibold))
          Text(capability.pluginDisplayName)
            .font(.caption2)
            .foregroundColor(.secondary)
        }

        Spacer()
      }

      if !capability.permissions.isEmpty {
        Text("Needs: \(PluginPermissionDisplay.summary(capability.permissions))")
          .font(.caption2)
          .foregroundColor(.secondary)
          .textSelection(.enabled)
      }

      if let reviewSummary = PluginCapabilityPresenter.reviewSummary(capability) {
        Text(reviewSummary)
          .font(.caption2)
          .foregroundColor(.secondary)
          .textSelection(.enabled)
      }

      if let diagnosticSummary = PluginCapabilityPresenter.diagnosticSummary(capability) {
        Text(diagnosticSummary)
          .font(.caption2)
          .foregroundColor(PluginCapabilityPresenter.diagnosticColor(capability))
          .textSelection(.enabled)
      }

      if let diagnosticDetail = PluginCapabilityPresenter.diagnosticDetail(capability) {
        Text("Needs attention: \(diagnosticDetail)")
          .font(.caption2)
          .foregroundColor(.orange)
          .textSelection(.enabled)
      }
    }
    .softPanel(
      tone: PluginCapabilityPresenter.diagnosticDetail(capability) == nil ? .neutral : .warning
    )
  }
}
