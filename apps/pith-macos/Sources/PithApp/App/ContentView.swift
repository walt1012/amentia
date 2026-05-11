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
      ToolbarItem {
        Button("Open Workspace") {
          viewModel.openWorkspace()
        }
        .disabled(!viewModel.canOpenWorkspace())
      }

      ToolbarItem {
        Button("New Thread") {
          viewModel.createThread()
        }
        .disabled(!viewModel.canCreateThread())
      }

      ToolbarItem(placement: .primaryAction) {
        if viewModel.shouldShowRuntimeToolbarAction() {
          Button(viewModel.runtimeLaunchButtonTitle()) {
            viewModel.launchRuntime()
          }
          .disabled(!viewModel.canLaunchRuntime())
        }
      }
    }
  }

  private var sidebar: some View {
    List(selection: Binding(get: { viewModel.selectedThreadID }, set: { viewModel.selectThread(id: $0) })) {
      Section("Threads") {
        if viewModel.threads.isEmpty {
          SidebarEmptyState(
            title: "No Threads Yet",
            detail: "Use the timeline setup flow to launch the runtime, choose a local model, and open a workspace."
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
  var body: some View {
    Form {
      Section("Model") {
        Text("First-use catalog: LFM2.5-350M default, Granite 4.0-H-350M recommended alternative")
      }

      Section("Platform") {
        Text("Target: macOS 12+ on Intel")
      }
    }
    .padding(20)
    .frame(width: 420)
  }
}
