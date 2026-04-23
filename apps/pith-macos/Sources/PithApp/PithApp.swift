import SwiftUI

@main
struct PithApp: App {
  @StateObject private var viewModel = AppViewModel()

  var body: some Scene {
    WindowGroup {
      ContentView(viewModel: viewModel)
        .frame(minWidth: 1120, minHeight: 720)
    }
    Settings {
      SettingsView()
    }
  }
}
