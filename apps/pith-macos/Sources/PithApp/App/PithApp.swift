import SwiftUI

@main
struct PithApp: App {
  @StateObject private var viewModel = AppViewModel()

  var body: some Scene {
    WindowGroup {
      ContentView(viewModel: viewModel)
        .frame(minWidth: 1120, minHeight: 720)
    }
    .commands {
      CommandGroup(replacing: .newItem) {
        Button("New Thread") {
          viewModel.createThread()
        }
        .keyboardShortcut("n", modifiers: [.command])
        .disabled(!viewModel.canCreateThread())
      }

      CommandMenu("Workflow") {
        Button(viewModel.runtimeLaunchButtonTitle()) {
          viewModel.launchRuntime()
        }
        .keyboardShortcut("r", modifiers: [.command])
        .disabled(!viewModel.canLaunchRuntime())

        Divider()

        Button("Open Workspace") {
          viewModel.openWorkspace()
        }
        .keyboardShortcut("o", modifiers: [.command])
        .disabled(!viewModel.canOpenWorkspace())

        Button("Install Plugin") {
          viewModel.installPlugin()
        }
        .keyboardShortcut("i", modifiers: [.command, .shift])
        .disabled(!viewModel.canInstallPlugin())

        Divider()

        Button("Send Message") {
          viewModel.sendDraftMessage()
        }
        .keyboardShortcut(.return, modifiers: [.command])
        .disabled(!viewModel.canSendDraftMessage())

        Button("Cancel Execution") {
          viewModel.cancelActiveTurn()
        }
        .keyboardShortcut(.cancelAction)
        .disabled(!viewModel.canCancelActiveTurn())
      }
    }
    Settings {
      SettingsView()
    }
  }
}
