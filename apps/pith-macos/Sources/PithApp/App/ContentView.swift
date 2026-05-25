import SwiftUI

struct ContentView: View {
  @ObservedObject var viewModel: AppViewModel

  var body: some View {
    NavigationView {
      sidebar
      TimelinePane(viewModel: viewModel)
      InspectorPane(viewModel: viewModel)
    }
    .toolbar {
      ToolbarItemGroup {
        if viewModel.shouldShowRuntimeLaunchToolbarAction(),
           viewModel.canLaunchRuntime()
        {
          Button(viewModel.runtimeLaunchButtonTitle()) {
            viewModel.launchRuntime()
          }
        }

        if let modelActionTitle = viewModel.modelSetupCalloutActionTitle(),
           viewModel.canRunModelSetupCalloutAction()
        {
          Button(modelActionTitle) {
            viewModel.runModelSetupCalloutAction()
          }
        }

        if viewModel.canOpenWorkspace() {
          Button("Open Workspace") {
            viewModel.openWorkspace()
          }
        }

        if viewModel.canCreateThread() {
          Button("New Thread") {
            viewModel.createThread()
          }
        }
      }
    }
    .onAppear {
      viewModel.startDailyUseSessionIfNeeded()
    }
  }

  private var sidebar: some View {
    List(selection: Binding(get: { viewModel.selectedThreadID }, set: { viewModel.selectThread(id: $0) })) {
      Section("Threads") {
        if viewModel.threads.isEmpty {
          SidebarEmptyState(
            title: "No Threads Yet",
            detail: viewModel.sidebarEmptyStateDetail()
          )
        } else {
          ForEach(viewModel.threads) { thread in
            VStack(alignment: .leading, spacing: 4) {
              Text(thread.title)
                .font(.headline)
              Text(thread.preview)
                .font(.caption)
                .foregroundColor(.secondary)
            }
            .padding(.vertical, 4)
            .tag(thread.id)
          }
        }
      }
    }
    .frame(minWidth: 240)
    .listStyle(.sidebar)
  }

}

private extension AppViewModel {
  func sidebarEmptyStateDetail() -> String {
    "\(setupProgressSummary()). \(setupProgressDetail())."
  }
}

private struct SidebarEmptyState: View {
  let title: String
  let detail: String

  var body: some View {
    VStack(alignment: .leading, spacing: 6) {
      Text(title)
        .font(.subheadline.weight(.semibold))
      Text(detail)
        .font(.caption)
        .foregroundColor(.secondary)
        .fixedSize(horizontal: false, vertical: true)
    }
    .padding(.vertical, 8)
  }
}

struct SettingsView: View {
  private let distributionTrust = DistributionTrustPresenter.summary()

  var body: some View {
    Form {
      Section("Local Models") {
        Text("Pith downloads verified GGUF models in app.")
        Text("Default: LFM2.5-350M. Alternative: Granite 4.0-H-350M.")
      }

      Section("Platform") {
        Text("Built for macOS 12+ on Intel.")
      }

      Section("Distribution") {
        Text(distributionTrust.title)
        Text(distributionTrust.detail)
      }
    }
    .padding(20)
    .frame(width: 420)
  }
}
