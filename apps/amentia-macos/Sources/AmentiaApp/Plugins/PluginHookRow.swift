import SwiftUI

struct PluginHookRow: View {
  let hook: PluginHookSummary
  let canRefresh: Bool
  let canDisablePlugin: Bool
  let onRevealSource: () -> Void
  let onRefresh: () -> Void
  let onDisablePlugin: () -> Void

  var body: some View {
    VStack(alignment: .leading, spacing: 6) {
      HStack(alignment: .top, spacing: 12) {
        VStack(alignment: .leading, spacing: 2) {
          Text(hook.title)
            .font(.caption.weight(.semibold))
          Text(PluginHookDisplay.statusLine(hook))
            .font(.caption2)
            .foregroundColor(hook.status == "ready" ? .secondary : .orange)
        }

        Spacer()

        if hook.status != "ready" {
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

        Button(PluginHookDisplay.disableButtonTitle(hook)) {
          onDisablePlugin()
        }
        .font(.caption2)
        .disabled(!canDisablePlugin)
        .help(PluginHookDisplay.disableDetail(hook))
      }

      Text(hook.description)
        .font(.caption2)
        .foregroundColor(.secondary)

      Text(PluginHookDisplay.disableDetail(hook))
        .font(.caption2)
        .foregroundColor(.secondary)
        .textSelection(.enabled)

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
