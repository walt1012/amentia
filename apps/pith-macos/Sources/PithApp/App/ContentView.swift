import SwiftUI

struct ContentView: View {
  @ObservedObject var viewModel: AppViewModel

  var body: some View {
    NavigationView {
      SessionSidebarView(viewModel: viewModel)
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
  }
}
