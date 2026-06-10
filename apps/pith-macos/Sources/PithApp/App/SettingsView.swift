import SwiftUI

struct SettingsView: View {
  @ObservedObject var viewModel: AppViewModel
  @State private var confirmsLocalDataDelete = false
  private let distributionTrust = DistributionTrustPresenter.summary()

  var body: some View {
    Form {
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
    .background(PithVisualStyle.paneBackground)
    .frame(width: 460)
  }

  private var localExecutionSection: some View {
    Section("Local Execution") {
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
    }
  }

  private var localModelSection: some View {
    Section("Local Models") {
      Text("Pith downloads and verifies one local model in app.")
      Text("Default: LFM2.5-350M. Alternatives: Granite 4.0-H-350M and MiniCPM5-1B.")
    }
  }

  private var platformSection: some View {
    Section("Platform") {
      Text("Built for macOS 12+ on Intel.")
    }
  }

  private var distributionSection: some View {
    Section("Distribution") {
      Text(distributionTrust.title)
      Text(distributionTrust.detail)
    }
  }
}

private struct SettingsStorageSection: View {
  let summary: LocalDataSettingsSummary
  let reveal: () -> Void
  let delete: () -> Void
  @Binding var confirmsLocalDataDelete: Bool

  var body: some View {
    Section("Storage") {
      Text(summary.storageSummary)
      Text(summary.ownershipDetail)
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

        Spacer()

        Button(summary.deleteButtonTitle, role: .destructive) {
          confirmsLocalDataDelete = true
        }
        .disabled(!summary.canDeleteLocalData)
      }
    }
    .alert(summary.confirmationTitle, isPresented: $confirmsLocalDataDelete) {
      Button("Delete Local Data", role: .destructive) {
        delete()
      }
      Button("Cancel", role: .cancel) {}
    } message: {
      Text(summary.confirmationMessage)
    }
  }
}
