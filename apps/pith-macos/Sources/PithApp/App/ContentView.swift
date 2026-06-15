import SwiftUI

struct ContentView: View {
  @ObservedObject var viewModel: AppViewModel
  @State private var sessionDeleteCandidate: ThreadSummary?
  @State private var sessionRevertCandidate: SessionRevertCandidate?

  var body: some View {
    NavigationView {
      SessionSidebarView(
        viewModel: viewModel,
        sessionDeleteCandidate: $sessionDeleteCandidate,
        sessionRevertCandidate: $sessionRevertCandidate
      )
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
          Button("Open Project") {
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
      let prompt = SessionChangePresenter.deletePrompt()
      return Alert(
        title: Text(prompt.title),
        message: Text(prompt.message),
        primaryButton: .destructive(Text(prompt.confirmButtonTitle)) {
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
}
