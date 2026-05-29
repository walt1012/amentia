import SwiftUI
import AppKit

@main
struct PithApp: App {
  @NSApplicationDelegateAdaptor(PithAppDelegate.self) private var appDelegate
  @StateObject private var viewModel = AppViewModel()

  var body: some Scene {
    WindowGroup {
      ContentView(viewModel: viewModel)
        .frame(minWidth: 1120, minHeight: 720)
        .background(MainWindowMarker())
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

        if let modelActionTitle = viewModel.modelSetupCalloutActionTitle() {
          Button(modelActionTitle) {
            viewModel.runModelSetupCalloutAction()
          }
          .keyboardShortcut("m", modifiers: [.command, .shift])
          .disabled(!viewModel.canRunModelSetupCalloutAction())
        }

        if let modelSecondaryActionTitle = viewModel.modelSetupCalloutSecondaryActionTitle() {
          Button(modelSecondaryActionTitle) {
            viewModel.runSetupCalloutSecondaryAction()
          }
          .disabled(!viewModel.canRunSetupCalloutSecondaryAction())
        }

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
      SettingsView(viewModel: viewModel)
    }
  }
}

private struct MainWindowMarker: NSViewRepresentable {
  func makeNSView(context: Context) -> NSView {
    NSView(frame: .zero)
  }

  func updateNSView(_ nsView: NSView, context: Context) {
    DispatchQueue.main.async {
      guard let window = nsView.window else {
        return
      }

      PithAppDelegate.registerMainWindow(window)
    }
  }
}
