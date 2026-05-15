import SwiftUI

struct PluginManagerPanel: View {
  @ObservedObject var viewModel: AppViewModel

  var body: some View {
    VStack(alignment: .leading, spacing: 10) {
      Text(viewModel.pluginCountSummary())
        .font(.headline)

      HStack(alignment: .firstTextBaseline, spacing: 8) {
        Text(pluginSurfaceSummary)
          .font(.caption)
          .foregroundColor(.secondary)

        Spacer()

        Button("Install Local Plugin") {
          viewModel.installPlugin()
        }
        .buttonStyle(.bordered)
        .disabled(!viewModel.canInstallPlugin())
      }

      Picker("Surface", selection: $viewModel.pluginManagerSection) {
        ForEach(PluginManagerSection.allCases) { section in
          Text(section.rawValue)
            .tag(section)
        }
      }
      .pickerStyle(.menu)

      PluginManagerSectionView(
        viewModel: viewModel,
        section: viewModel.pluginManagerSection
      )
    }
    .frame(maxWidth: .infinity, alignment: .leading)
  }

  private var pluginSurfaceSummary: String {
    [
      viewModel.pluginRegistryCountSummary(),
      viewModel.pluginConnectorCountSummary(),
      viewModel.pluginCommandCountSummary(),
      viewModel.pluginHookCountSummary(),
    ].joined(separator: " | ")
  }
}
