import SwiftUI

struct SettingsView: View {
  @ObservedObject var viewModel: AppViewModel
  @State private var confirmsLocalDataDelete = false
  private let distributionTrust = DistributionTrustPresenter.summary()

  var body: some View {
    ScrollView {
      VStack(alignment: .leading, spacing: 14) {
        settingsHeader
        localExecutionSection
        localModelSection
        SettingsStorageSection(
          summary: viewModel.localDataSettingsSummary(),
          reveal: viewModel.revealLocalDataFolder,
          delete: viewModel.deleteLocalData,
          confirmsLocalDataDelete: $confirmsLocalDataDelete
        )
        platformSection
        distributionSection
      }
      .padding(20)
    }
    .background(PithVisualStyle.paneBackground)
    .frame(width: 500)
    .frame(minHeight: 560)
  }

  private var settingsHeader: some View {
    VStack(alignment: .leading, spacing: 6) {
      Text("Pith Settings")
        .font(.title3.weight(.semibold))
      Text("Local-first cowork setup, safety, storage, and release trust.")
        .font(.caption)
        .foregroundColor(.secondary)
    }
    .frame(maxWidth: .infinity, alignment: .leading)
  }

  private var localExecutionSection: some View {
    SettingsCard(title: "Action Safety", systemImage: "shield", tone: .active) {
      Picker(
        "Mode",
        selection: Binding(
          get: { viewModel.selectedLocalExecutionSafetyMode },
          set: { viewModel.selectLocalExecutionSafetyMode($0) }
        )
      ) {
        ForEach(LocalExecutionSafetyModePresenter.modes, id: \.self) { mode in
          Text(LocalExecutionSafetyModePresenter.userTitle(mode))
            .tag(mode)
        }
      }
      Text(LocalExecutionSafetyModePresenter.userDetail(
        viewModel.selectedLocalExecutionSafetyMode
      ))
      .font(.caption)
      .foregroundColor(.secondary)
      .fixedSize(horizontal: false, vertical: true)
    }
  }

  private var localModelSection: some View {
    SettingsCard(title: "Local Models", systemImage: "cpu", tone: .ready) {
      Text("Pith downloads and verifies one local model in app.")
        .font(.caption)
      Text("Default: LFM2.5-350M. Alternatives: Granite 4.0-H-350M and MiniCPM5-1B.")
        .font(.caption)
        .foregroundColor(.secondary)
      Text("Reset Pith removes downloaded models and starts setup fresh.")
        .font(.caption2)
        .foregroundColor(.secondary)
    }
  }

  private var platformSection: some View {
    SettingsCard(title: "Platform", systemImage: "desktopcomputer", tone: .neutral) {
      Text("Built for macOS 12+ on Intel.")
        .font(.caption)
    }
  }

  private var distributionSection: some View {
    SettingsCard(title: "Distribution", systemImage: "checkmark.seal", tone: distributionTone) {
      HStack(alignment: .firstTextBaseline, spacing: 8) {
        Text(distributionTrust.title)
          .font(.caption.weight(.semibold))
        StatusPill(label: distributionTrustLabel, tone: distributionTone)
      }
      Text(distributionTrust.summary)
        .font(.caption)
        .foregroundColor(.secondary)
      Text(distributionTrust.detail)
        .font(.caption2)
        .foregroundColor(.secondary)
        .fixedSize(horizontal: false, vertical: true)

      DisclosureGroup("Advanced") {
        Text(distributionTrust.advancedDetail)
          .font(.caption)
          .foregroundColor(.secondary)
          .textSelection(.enabled)
      }
    }
  }

  private var distributionTone: StatusTone {
    switch distributionTrust.title {
    case "Verified Installer":
      return .ready
    case "Manual Open Required", "Local Development Build":
      return .warning
    default:
      return .neutral
    }
  }

  private var distributionTrustLabel: String {
    switch distributionTrust.title {
    case "Verified Installer":
      return "Verified"
    case "Manual Open Required":
      return "Manual Open"
    case "Local Development Build":
      return "Local Build"
    default:
      return "Development"
    }
  }
}

private struct SettingsCard<Content: View>: View {
  let title: String
  let systemImage: String
  let tone: StatusTone
  let content: Content

  init(
    title: String,
    systemImage: String,
    tone: StatusTone,
    @ViewBuilder content: () -> Content
  ) {
    self.title = title
    self.systemImage = systemImage
    self.tone = tone
    self.content = content()
  }

  var body: some View {
    HStack(alignment: .top, spacing: 12) {
      ZStack {
        Circle()
          .fill(tone.color.opacity(0.11))
          .frame(width: 32, height: 32)
        Image(systemName: systemImage)
          .font(.body.weight(.semibold))
          .foregroundColor(tone.color)
      }

      VStack(alignment: .leading, spacing: 8) {
        Text(title)
          .font(.headline.weight(.semibold))
        content
      }
      .frame(maxWidth: .infinity, alignment: .leading)
    }
    .softPanel(tone: tone)
  }
}

private struct SettingsStorageSection: View {
  let summary: LocalDataSettingsSummary
  let reveal: () -> Void
  let delete: () -> Void
  @Binding var confirmsLocalDataDelete: Bool

  var body: some View {
    SettingsCard(title: "Storage", systemImage: "externaldrive", tone: .neutral) {
      Text(summary.storageSummary)
        .font(.caption)
      Text(summary.ownershipDetail)
        .font(.caption)
        .foregroundColor(.secondary)
      Text(summary.uninstallDetail)
        .font(.caption2)
        .foregroundColor(.secondary)
      if let blockedDetail = summary.blockedDetail {
        Text(blockedDetail)
          .font(.caption)
          .foregroundColor(.secondary)
      }

      DisclosureGroup("Advanced") {
        Text(summary.localDataPath)
          .font(.caption.monospaced())
          .foregroundColor(.secondary)
          .textSelection(.enabled)
      }

      HStack {
        Button(summary.revealButtonTitle) {
          reveal()
        }
        .controlSize(.small)

        Spacer()

        Button(summary.deleteButtonTitle, role: .destructive) {
          confirmsLocalDataDelete = true
        }
        .controlSize(.small)
        .disabled(!summary.canDeleteLocalData)
      }
    }
    .alert(summary.confirmationTitle, isPresented: $confirmsLocalDataDelete) {
      Button("Reset Pith", role: .destructive) {
        delete()
      }
      Button("Cancel", role: .cancel) {}
    } message: {
      Text(summary.confirmationMessage)
    }
  }
}
