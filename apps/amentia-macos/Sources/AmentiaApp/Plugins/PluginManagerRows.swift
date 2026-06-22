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

struct PluginSkillRow: View {
  let skill: PluginSkillSummary
  let canDisablePlugin: Bool
  let onRevealSource: () -> Void
  let onDisablePlugin: () -> Void

  var body: some View {
    VStack(alignment: .leading, spacing: 6) {
      HStack(alignment: .top, spacing: 12) {
        VStack(alignment: .leading, spacing: 2) {
          Text(skill.description)
            .font(.caption.weight(.semibold))
          Text("\(skill.pluginDisplayName) | \(displayStatus)")
            .font(.caption2)
            .foregroundColor(.secondary)
        }

        Spacer()

        HStack(spacing: 8) {
          Button("Show File") {
            onRevealSource()
          }
          .buttonStyle(.bordered)
          .controlSize(.small)

          Button(PluginSkillDisplay.disableButtonTitle(skill)) {
            onDisablePlugin()
          }
          .buttonStyle(.bordered)
          .controlSize(.small)
          .disabled(!canDisablePlugin)
          .help(PluginSkillDisplay.disableDetail(skill))
        }
      }

      Text(PluginSkillDisplay.disableDetail(skill))
        .font(.caption2)
        .foregroundColor(.secondary)
        .textSelection(.enabled)

      if !skill.permissions.isEmpty {
        Text("Needs: \(PluginPermissionDisplay.summary(skill.permissions))")
          .font(.caption2)
          .foregroundColor(.secondary)
          .textSelection(.enabled)
      }

      if let previewLine {
        Text("Preview: \(previewLine)")
          .font(.caption2)
          .foregroundColor(.secondary)
          .textSelection(.enabled)
      }

      if let issueText {
        Text(issueText)
          .font(.caption2)
          .foregroundColor(.orange)
          .textSelection(.enabled)
      }
    }
    .softPanel(tone: skill.status == "ready" ? .neutral : .warning)
  }

  private var displayStatus: String {
    PluginStatusDisplay.skillStatus(skill.status).capitalized
  }

  private var previewLine: String? {
    PluginSkillDisplay.previewLine(skill.preview, maxLength: 140)
  }

  private var issueText: String? {
    PluginSkillDisplay.issueText(skill)
  }
}
