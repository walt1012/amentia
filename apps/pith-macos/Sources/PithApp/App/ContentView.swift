import SwiftUI

struct ContentView: View {
  @ObservedObject var viewModel: AppViewModel
  @State private var sessionDeleteCandidate: ThreadSummary?
  @State private var sessionRevertCandidate: SessionRevertCandidate?

  var body: some View {
    NavigationView {
      sidebar
      TimelinePane(viewModel: viewModel)
      InspectorPane(viewModel: viewModel)
    }
    .background(PithVisualStyle.windowBackground)
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
          Button("New Session") {
            viewModel.createThread()
          }
        }
      }
    }
    .onAppear {
      viewModel.startDailyUseSessionIfNeeded()
    }
    .alert(item: $sessionDeleteCandidate) { thread in
      Alert(
        title: Text("Delete Session?"),
        message: Text(
          "Pith will delete this session's messages, timeline, and pending approvals. Workspace files and repositories will not be changed."
        ),
        primaryButton: .destructive(Text("Delete Session")) {
          viewModel.deleteThread(thread)
        },
        secondaryButton: .cancel()
      )
    }
    .alert(item: $sessionRevertCandidate) { candidate in
      let prompt = SessionChangePresenter.revertPrompt(for: candidate.preview)
      if !prompt.allowsRevert {
        return Alert(
          title: Text(prompt.title),
          message: Text(prompt.message),
          dismissButton: .default(Text("OK"))
        )
      }

      return Alert(
        title: Text(prompt.title),
        message: Text(prompt.message),
        primaryButton: .destructive(Text(prompt.confirmButtonTitle)) {
          viewModel.revertThreadChanges(candidate.thread)
        },
        secondaryButton: .cancel()
      )
    }
  }

  private var sidebar: some View {
    List(selection: Binding(get: { viewModel.selectedThreadID }, set: { viewModel.selectThread(id: $0) })) {
      Section("Sessions") {
        if viewModel.threads.isEmpty {
          SidebarEmptyState(
            title: "No Sessions Yet",
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
            .contextMenu {
              Button("Revert Session Changes...") {
                Task {
                  if let preview = await viewModel.previewThreadChanges(thread) {
                    sessionRevertCandidate = SessionRevertCandidate(
                      thread: thread,
                      preview: preview
                    )
                  }
                }
              }
              .disabled(!viewModel.canRevertThreadChanges(thread))

              Button("Delete Session...", role: .destructive) {
                sessionDeleteCandidate = thread
              }
              .disabled(!viewModel.canDeleteThread(thread))
            }
          }
        }
      }
    }
    .frame(minWidth: 240)
    .listStyle(.sidebar)
    .background(PithVisualStyle.windowBackground)
  }

}

private struct SessionRevertCandidate: Identifiable {
  let thread: ThreadSummary
  let preview: RuntimeBridge.RuntimeThreadChangePreview

  var id: String {
    thread.id
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
